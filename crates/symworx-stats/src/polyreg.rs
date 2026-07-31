// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Univariate polynomial regression and degree search.
//!
//! Fits `y ≈ β₀ + β₁ x + β₂ x² + … + β_d x^d` for degrees `0..=max_degree`
//! (subject to sample-size limits) and returns coefficients plus fit quality
//! for each degree — useful for teaching model order selection.
//!
//! ## Sample-size rules
//!
//! For degree `d` there are `n_params = d + 1` free coefficients.
//!
//! | Rule | Behavior |
//! |------|----------|
//! | **Hard** | If `n_samples < d + 1`, degree `d` (and higher) are **not** fitted. |
//! | **Soft** | If `n_samples < 2 · (d + 1)`, a warning string is recorded on the result. |
//!
//! Requesting `max_degree = k` with only `n = k − 1` samples therefore stops
//! at degree `n − 1 = k − 2` and never returns a degree-`k` fit.
//!
//! Warnings are always collected in [`PolynomialDegreeSearch::warnings`].
//! Printing via `eprintln!` is **opt-in** (`print_warnings`); libraries stay quiet
//! by default — demos/CLIs can print `search.warnings` themselves.
//!
//! Requires the `linalg` feature (uses [`crate::ols`] — does **not** reimplement
//! the linear solver; only builds the Vandermonde-style design and calls OLS).
//!
//! Residuals are optional (see [`PolynomialSearchConfig::return_residuals`]) so
//! the default path stays light when you only need β and aggregate metrics.

use std::fmt;

use ndarray::{
    Array1,
    Array2,
};

use crate::{
    error_metrics::{
        RegressionReport,
        regression_report,
        residuals,
    },
    linreg::{
        LinearModel,
        ols,
    },
    model_select::{
        ModelFitScores,
        NestedModelTest,
        nested_lr_chi2,
        rss,
    },
};

/// Options for [`fit_polynomial_degrees_with`].
#[derive(Debug, Clone)]
pub struct PolynomialSearchConfig {
    /// Highest degree to attempt (`0..=max_degree`, subject to sample-size caps).
    pub max_degree: usize,
    /// If `true`, each [`PolynomialDegreeFit`] includes in-sample residuals
    /// `e = y − ŷ`. Default **`false`** (not standard output — request when needed).
    pub return_residuals: bool,
    /// If `true`, also emit soft/hard sample-size messages with `eprintln!`.
    /// Default **`false`** — callers should print [`PolynomialDegreeSearch::warnings`].
    pub print_warnings: bool,
}

impl Default for PolynomialSearchConfig {
    fn default() -> Self {
        Self {
            max_degree: 3,
            return_residuals: false,
            print_warnings: false,
        }
    }
}

#[cfg(feature = "linalg")]
fn push_warning(warnings: &mut Vec<String>, msg: String, print: bool) {
    if print {
        eprintln!("symworx_stats::polyreg warning: {msg}");
    }
    warnings.push(msg);
}

/// One fitted polynomial degree.
#[derive(Debug, Clone)]
pub struct PolynomialDegreeFit {
    /// Polynomial degree `d`.
    pub degree: usize,
    /// Number of free parameters (`degree + 1`).
    pub n_params: usize,
    /// Fitted model: `intercept` is β₀; `coefficients[j]` is β_{j+1} for x^{j+1}.
    pub model: LinearModel,
    /// In-sample regression report (y vs ŷ).
    pub report: RegressionReport,
    /// In-sample residuals `eᵢ = yᵢ − ŷᵢ`, only if
    /// [`PolynomialSearchConfig::return_residuals`] was set; otherwise `None`.
    pub residuals: Option<Vec<f64>>,
    /// AIC / BIC / adj-R² / RSS for selection (always filled when fit succeeds).
    pub scores: ModelFitScores,
    /// Nested LR χ² vs the previous fitted degree (`d−1` when present).
    pub chi2_vs_prev: Option<f64>,
    /// df for [`Self::chi2_vs_prev`] (usually 1).
    pub chi2_vs_prev_df: usize,
    /// Approximate p-value for [`Self::chi2_vs_prev`].
    pub chi2_vs_prev_p: Option<f64>,
    /// Nested LR χ² vs degree-0 (intercept-only), if that fit exists.
    pub chi2_vs_null: Option<f64>,
    /// Degrees of freedom for [`Self::chi2_vs_null`].
    pub chi2_vs_null_df: usize,
    /// Approximate p-value for [`Self::chi2_vs_null`].
    pub chi2_vs_null_p: Option<f64>,
}

impl PolynomialDegreeFit {
    /// Packed coefficients `[β₀, β₁, …, β_d]`.
    pub fn coeffs_packed(&self) -> Array1<f64> {
        self.model.to_packed()
    }

    /// Predict at new scalar x values.
    pub fn predict(&self, x: &[f64]) -> Array1<f64> {
        let design = polynomial_design(x, self.degree);
        self.model.predict(&design)
    }

    /// Compute residuals for arbitrary `y` / predictions (does not use stored field).
    pub fn residuals_of(&self, y: &[f64], yhat: &[f64]) -> Vec<f64> {
        residuals(y, yhat)
    }
}

/// Result of sweeping polynomial degrees.
#[derive(Debug, Clone)]
pub struct PolynomialDegreeSearch {
    /// Sample size used.
    pub n_samples: usize,
    /// Requested maximum degree.
    pub max_degree_requested: usize,
    /// Highest degree actually fitted (`min(max_degree, n−1)` when n≥1).
    pub max_degree_fitted: usize,
    /// Fits for each degree that was successfully estimated (ascending degree).
    pub fits: Vec<PolynomialDegreeFit>,
    /// Soft warnings (sample size heuristics, truncated max degree, …).
    pub warnings: Vec<String>,
}

impl PolynomialDegreeSearch {
    /// Degree with highest in-sample R² among successful fits (ties → lower degree).
    ///
    /// **Prefer [`Self::best_degree_by_aic`] or [`Self::best_degree_by_bic`] for
    /// selection** — R² alone almost always favors the highest degree.
    pub fn best_degree_by_r2(&self) -> Option<usize> {
        self.best_by(|f| f.report.r2, true)
    }

    /// Degree with **lowest AIC** (ties → lower degree).
    pub fn best_degree_by_aic(&self) -> Option<usize> {
        self.best_by(|f| f.scores.aic, false)
    }

    /// Degree with **lowest BIC** (ties → lower degree). Stronger penalty than AIC.
    pub fn best_degree_by_bic(&self) -> Option<usize> {
        self.best_by(|f| f.scores.bic, false)
    }

    /// Degree with highest adjusted R² (ties → lower degree).
    pub fn best_degree_by_adj_r2(&self) -> Option<usize> {
        self.best_by(|f| f.scores.adj_r2, true)
    }

    fn best_by(&self, score: impl Fn(&PolynomialDegreeFit) -> f64, maximize: bool) -> Option<usize> {
        let mut best: Option<&PolynomialDegreeFit> = None;
        for f in &self.fits {
            let s = score(f);
            if !s.is_finite() {
                continue;
            }
            best = Some(match best {
                None => f,
                Some(b) => {
                    let sb = score(b);
                    let better = if maximize { s > sb + 1e-15 } else { s < sb - 1e-15 };
                    let tie = (s - sb).abs() <= 1e-15;
                    if better || (tie && f.degree < b.degree) { f } else { b }
                }
            });
        }
        best.map(|f| f.degree)
    }

    /// Fit for a specific degree, if present.
    pub fn fit_for_degree(&self, degree: usize) -> Option<&PolynomialDegreeFit> {
        self.fits.iter().find(|f| f.degree == degree)
    }

    /// Nested test of `degree_alt` vs `degree_null` (must both be fitted; alt > null).
    pub fn nested_test(&self, degree_null: usize, degree_alt: usize) -> Option<NestedModelTest> {
        if degree_alt <= degree_null {
            return None;
        }
        let n = self.n_samples;
        let a = self.fit_for_degree(degree_null)?;
        let b = self.fit_for_degree(degree_alt)?;
        Some(NestedModelTest::from_rss(
            n,
            a.scores.rss,
            a.n_params,
            a.report.r2,
            b.scores.rss,
            b.n_params,
            b.report.r2,
        ))
    }
}

impl fmt::Display for PolynomialDegreeSearch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "PolynomialDegreeSearch n={} requested_max={} fitted_max={} n_fits={}",
            self.n_samples,
            self.max_degree_requested,
            self.max_degree_fitted,
            self.fits.len()
        )?;
        for fit in &self.fits {
            writeln!(
                f,
                "  degree={}: n_params={}  R²={:.6}  RMSE={:.6}  β={:?}",
                fit.degree,
                fit.n_params,
                fit.report.r2,
                fit.report.rmse,
                fit.coeffs_packed().to_vec()
            )?;
        }
        for w in &self.warnings {
            writeln!(f, "  warning: {w}")?;
        }
        Ok(())
    }
}

/// Errors from [`fit_polynomial_degrees`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolyRegError {
    /// Empty `x` or `y`.
    EmptyData,
    /// `x` and `y` lengths differ.
    LengthMismatch {
        /// Length of x.
        n_x: usize,
        /// Length of y.
        n_y: usize,
    },
    /// No degree could be fitted (e.g. `n = 0` after checks).
    NoFeasibleDegree {
        /// Sample size.
        n_samples: usize,
        /// Requested max degree.
        max_degree: usize,
    },
}

impl fmt::Display for PolyRegError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolyRegError::EmptyData => write!(f, "x and y must be non-empty"),
            PolyRegError::LengthMismatch { n_x, n_y } => {
                write!(f, "x length {n_x} != y length {n_y}")
            }
            PolyRegError::NoFeasibleDegree { n_samples, max_degree } => write!(
                f,
                "no feasible polynomial degree for n={n_samples}, max_degree={max_degree}"
            ),
        }
    }
}

impl std::error::Error for PolyRegError {}

/// Build design matrix with columns `[x, x², …, x^d]` (no intercept column;
/// [`ols`] adds intercept). Degree 0 → empty feature matrix with `n` rows.
pub fn polynomial_design(x: &[f64], degree: usize) -> Array2<f64> {
    let n = x.len();
    if degree == 0 {
        return Array2::zeros((n, 0));
    }
    let mut design = Array2::<f64>::zeros((n, degree));
    for i in 0..n {
        let mut pow = x[i];
        for d in 0..degree {
            design[[i, d]] = pow;
            pow *= x[i];
        }
    }
    design
}

/// Maximum polynomial degree that can be fitted with `n` samples
/// (`n_params = d+1 ≤ n` ⇒ `d ≤ n−1`).
#[inline]
pub fn max_feasible_degree(n_samples: usize) -> Option<usize> {
    if n_samples == 0 { None } else { Some(n_samples - 1) }
}

/// Soft rule: prefer at least two samples per free parameter.
#[inline]
pub fn soft_min_samples_for_degree(degree: usize) -> usize {
    2 * (degree + 1)
}

/// Fit polynomials of degree `0, 1, …, max_degree` (truncated by sample size).
///
/// Convenience wrapper around [`fit_polynomial_degrees_with`] with residuals
/// off and `print_warnings = false` (warnings only on the returned struct).
///
/// # Hard stop
/// Degree `d` requires `n ≥ d + 1`. Higher degrees are skipped (warning recorded).
/// If `n = k − 1` and `max_degree = k`, degree `k` is **not** returned.
///
/// # Soft warning
/// For each fitted degree with `n < 2·(d+1)`, a warning is recorded.
///
/// Requires `linalg` (via [`ols`]).
#[cfg(feature = "linalg")]
pub fn fit_polynomial_degrees(x: &[f64], y: &[f64], max_degree: usize) -> Result<PolynomialDegreeSearch, PolyRegError> {
    fit_polynomial_degrees_with(
        x,
        y,
        &PolynomialSearchConfig {
            max_degree,
            return_residuals: false,
            print_warnings: false,
        },
    )
}

/// Like [`fit_polynomial_degrees`], with explicit [`PolynomialSearchConfig`]
/// (optional residual vectors, max degree).
#[cfg(feature = "linalg")]
pub fn fit_polynomial_degrees_with(
    x: &[f64],
    y: &[f64],
    config: &PolynomialSearchConfig,
) -> Result<PolynomialDegreeSearch, PolyRegError> {
    let max_degree = config.max_degree;
    if x.is_empty() || y.is_empty() {
        return Err(PolyRegError::EmptyData);
    }
    if x.len() != y.len() {
        return Err(PolyRegError::LengthMismatch {
            n_x: x.len(),
            n_y: y.len(),
        });
    }

    let n = x.len();
    let mut warnings = Vec::new();

    let feasible_cap = max_feasible_degree(n).unwrap();
    let max_fit = max_degree.min(feasible_cap);

    if max_fit < max_degree {
        push_warning(
            &mut warnings,
            format!(
                "requested max_degree={max_degree} but n={n} only supports degree ≤ {feasible_cap} \
                 (need n ≥ d+1 free parameters); higher degrees omitted"
            ),
            config.print_warnings,
        );
    }

    // Explicit callout for n = k-1 when k was requested
    if n + 1 == max_degree {
        // n = k - 1 when max_degree = k
        push_warning(
            &mut warnings,
            format!(
                "n_samples={n} is max_degree−1 ({max_degree}−1): cannot identify a degree-{max_degree} \
                 polynomial (needs {need} points); stopped at degree {max_fit}",
                need = max_degree + 1
            ),
            config.print_warnings,
        );
    }

    if max_degree == 0 && n == 0 {
        return Err(PolyRegError::NoFeasibleDegree {
            n_samples: n,
            max_degree,
        });
    }

    let y_arr = Array1::from(y.to_vec());
    let mut fits = Vec::with_capacity(max_fit + 1);

    for d in 0..=max_fit {
        let n_params = d + 1;
        if n < n_params {
            // Hard stop — should not happen given max_fit = n-1
            break;
        }

        if n < soft_min_samples_for_degree(d) {
            push_warning(
                &mut warnings,
                format!(
                    "degree {d}: n={n} < 2×n_params={} (soft rule of thumb); fit is poorly determined",
                    soft_min_samples_for_degree(d)
                ),
                config.print_warnings,
            );
        }

        let design = polynomial_design(x, d);
        // Delegate solve to linreg::ols (does not reimplement normal equations here)
        let model = ols(&design, &y_arr);
        let yhat = model.predict(&design);
        let yhat_vec = yhat.to_vec();
        let report = regression_report(y, &yhat_vec);
        let rss_v = rss(y, &yhat_vec);
        let scores = ModelFitScores::from_rss(n, n_params, rss_v, report.r2);
        let res = if config.return_residuals {
            Some(residuals(y, &yhat_vec))
        } else {
            None
        };
        fits.push(PolynomialDegreeFit {
            degree: d,
            n_params,
            model,
            report,
            residuals: res,
            scores,
            chi2_vs_prev: None,
            chi2_vs_prev_df: 0,
            chi2_vs_prev_p: None,
            chi2_vs_null: None,
            chi2_vs_null_df: 0,
            chi2_vs_null_p: None,
        });
    }

    if fits.is_empty() {
        return Err(PolyRegError::NoFeasibleDegree {
            n_samples: n,
            max_degree,
        });
    }

    // Fill nested χ² comparisons (vs previous degree and vs intercept-only).
    fill_nested_chi2(&mut fits, n);

    Ok(PolynomialDegreeSearch {
        n_samples: n,
        max_degree_requested: max_degree,
        max_degree_fitted: fits.last().map(|f| f.degree).unwrap_or(0),
        fits,
        warnings,
    })
}

#[cfg(feature = "linalg")]
fn fill_nested_chi2(fits: &mut [PolynomialDegreeFit], n: usize) {
    use crate::model_select::chi2_sf;

    let null_rss = fits.first().map(|f| f.scores.rss);
    let null_k = fits.first().map(|f| f.n_params);
    let null_deg = fits.first().map(|f| f.degree);

    for i in 0..fits.len() {
        // vs previous degree in the sweep
        if i > 0 {
            let prev = &fits[i - 1];
            let cur = &fits[i];
            let df = cur.n_params.saturating_sub(prev.n_params);
            let chi = nested_lr_chi2(prev.scores.rss, cur.scores.rss, n);
            let p = if df > 0 && chi.is_finite() {
                Some(chi2_sf(chi, df as f64))
            } else {
                None
            };
            fits[i].chi2_vs_prev = Some(chi);
            fits[i].chi2_vs_prev_df = df;
            fits[i].chi2_vs_prev_p = p;
        }
        // vs degree-0 / first fit when this is richer
        if let (Some(r0), Some(k0), Some(d0)) = (null_rss, null_k, null_deg)
            && fits[i].degree > d0
        {
            let df = fits[i].n_params.saturating_sub(k0);
            let chi = nested_lr_chi2(r0, fits[i].scores.rss, n);
            let p = if df > 0 && chi.is_finite() {
                Some(chi2_sf(chi, df as f64))
            } else {
                None
            };
            fits[i].chi2_vs_null = Some(chi);
            fits[i].chi2_vs_null_df = df;
            fits[i].chi2_vs_null_p = p;
        }
    }
}

/// Stub without `linalg`.
#[cfg(not(feature = "linalg"))]
pub fn fit_polynomial_degrees(
    _x: &[f64],
    _y: &[f64],
    _max_degree: usize,
) -> Result<PolynomialDegreeSearch, PolyRegError> {
    panic!(
        "symworx_stats::fit_polynomial_degrees requires the `linalg` feature \
         (uses OLS). Enable features = [\"linalg\"] on symworx-stats."
    );
}

/// Stub without `linalg`.
#[cfg(not(feature = "linalg"))]
pub fn fit_polynomial_degrees_with(
    _x: &[f64],
    _y: &[f64],
    _config: &PolynomialSearchConfig,
) -> Result<PolynomialDegreeSearch, PolyRegError> {
    panic!(
        "symworx_stats::fit_polynomial_degrees_with requires the `linalg` feature \
         (uses OLS). Enable features = [\"linalg\"] on symworx-stats."
    );
}

#[cfg(all(test, feature = "linalg"))]
mod tests {
    use super::*;

    #[test]
    fn recovers_quadratic() {
        // y = 1 + 2x + 3x²
        let x: Vec<f64> = (0..20).map(|i| i as f64 * 0.1).collect();
        let y: Vec<f64> = x.iter().map(|&xi| 1.0 + 2.0 * xi + 3.0 * xi * xi).collect();
        let search = fit_polynomial_degrees(&x, &y, 4).unwrap();
        let fit2 = search.fit_for_degree(2).unwrap();
        let c = fit2.coeffs_packed();
        assert!((c[0] - 1.0).abs() < 1e-6, "β0 {:?}", c);
        assert!((c[1] - 2.0).abs() < 1e-6, "β1 {:?}", c);
        assert!((c[2] - 3.0).abs() < 1e-6, "β2 {:?}", c);
        assert!(fit2.report.r2 > 0.999);
        // higher degrees should still fit essentially perfectly on noiseless data
        assert!(search.fit_for_degree(3).unwrap().report.r2 > 0.999);
    }

    #[test]
    fn hard_stop_when_n_is_k_minus_1() {
        // n = 4, request max_degree k = 5 → n = k-1; max feasible degree = 3
        let x = vec![0.0, 1.0, 2.0, 3.0];
        let y = vec![1.0, 2.0, 3.0, 5.0];
        let search = fit_polynomial_degrees(&x, &y, 5).unwrap();
        assert_eq!(search.max_degree_fitted, 3);
        assert!(search.fit_for_degree(5).is_none());
        assert!(search.fit_for_degree(4).is_none());
        assert!(search.warnings.iter().any(|w| w.contains("omitted")
            || w.contains("max_degree−1")
            || w.contains("max_degree-1")
            || w.contains("degree-5")
            || w.contains("supports degree")));
    }

    #[test]
    fn soft_warning_when_n_lt_2k() {
        // degree 2 needs soft 2*3=6 samples; n=5 triggers soft warning
        let x: Vec<f64> = (0..5).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|xi| xi * xi).collect();
        let search = fit_polynomial_degrees(&x, &y, 2).unwrap();
        assert!(search.fit_for_degree(2).is_some());
        assert!(
            search
                .warnings
                .iter()
                .any(|w| w.contains("soft") || w.contains("2×") || w.contains("2x") || w.contains("poorly determined")),
            "warnings: {:?}",
            search.warnings
        );
    }

    #[test]
    fn design_columns() {
        let x = [2.0, 3.0];
        let d = polynomial_design(&x, 3);
        assert_eq!(d.nrows(), 2);
        assert_eq!(d.ncols(), 3);
        assert!((d[[0, 0]] - 2.0).abs() < 1e-15);
        assert!((d[[0, 1]] - 4.0).abs() < 1e-15);
        assert!((d[[0, 2]] - 8.0).abs() < 1e-15);
    }

    #[test]
    fn residuals_optional_default_none() {
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|t| 1.0 + 2.0 * t).collect();
        let search = fit_polynomial_degrees(&x, &y, 2).unwrap();
        for f in &search.fits {
            assert!(f.residuals.is_none());
        }
    }

    #[test]
    fn residuals_when_requested() {
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|t| 1.0 + 2.0 * t).collect();
        let search = fit_polynomial_degrees_with(
            &x,
            &y,
            &PolynomialSearchConfig {
                max_degree: 1,
                return_residuals: true,
                print_warnings: false,
            },
        )
        .unwrap();
        let fit = search.fit_for_degree(1).unwrap();
        let e = fit.residuals.as_ref().expect("residuals present");
        assert_eq!(e.len(), x.len());
        // exact line → residuals ~ 0
        assert!(e.iter().all(|v| v.abs() < 1e-9));
    }
}
