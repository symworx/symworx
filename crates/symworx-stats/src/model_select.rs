// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Model comparison / selection for continuous (Gaussian) linear fits.
//!
//! Raw **R² always rises** (or stays flat) when you add parameters in-sample,
//! so tiny gains at higher polynomial degrees do **not** justify a richer
//! model. Prefer penalized scores and nested tests:
//!
//! | Tool | Use |
//! |------|-----|
//! | **Adjusted R²** | R² with a parameter penalty (still in-sample) |
//! | **AIC / BIC** | Information criteria — lower is better; BIC penalizes more |
//! | **LR χ²** | Nested likelihood-ratio test (Gaussian RSS form) |
//! | **Nested F** | Exact small-sample nested test for linear models |
//!
//! For polynomials of degree `d` vs `d−1`, the fuller model is nested in the
//! reduced one (extra coefficients set to 0 under H₀).

use crate::error_metrics::mse;

/// Residual sum of squares `Σ (y − ŷ)²`.
///
/// Returns `f64::NAN` if lengths differ or inputs are empty.
pub fn rss(actual: &[f64], predicted: &[f64]) -> f64 {
    if actual.len() != predicted.len() || actual.is_empty() {
        return f64::NAN;
    }
    actual
        .iter()
        .zip(predicted.iter())
        .map(|(a, p)| {
            let e = a - p;
            e * e
        })
        .sum()
}

/// Adjusted R² = `1 − (1−R²)·(n−1)/(n−k)` with `k = n_params` (incl. intercept).
///
/// Returns `f64::NAN` if `n <= k` or `r2` is non-finite.
pub fn adjusted_r2(r2: f64, n: usize, n_params: usize) -> f64 {
    if !r2.is_finite() || n <= n_params || n < 2 {
        return f64::NAN;
    }
    let n = n as f64;
    let k = n_params as f64;
    1.0 - (1.0 - r2) * (n - 1.0) / (n - k)
}

/// Gaussian linear-model **AIC** (up to an additive constant shared by models
/// on the same data):
///
/// ```text
/// AIC = n · ln(RSS / n) + 2 k
/// ```
///
/// Lower is better. `n_params` includes the intercept.
pub fn aic_gaussian(rss: f64, n: usize, n_params: usize) -> f64 {
    if !rss.is_finite() || rss < 0.0 || n == 0 || n_params == 0 {
        return f64::NAN;
    }
    // Guard RSS=0 (perfect fit): use tiny floor so ln is defined.
    let rss = rss.max(f64::MIN_POSITIVE);
    let n = n as f64;
    let k = n_params as f64;
    n * (rss / n).ln() + 2.0 * k
}

/// Gaussian linear-model **BIC**:
///
/// ```text
/// BIC = n · ln(RSS / n) + k · ln(n)
/// ```
///
/// Lower is better; stronger complexity penalty than AIC for large `n`.
pub fn bic_gaussian(rss: f64, n: usize, n_params: usize) -> f64 {
    if !rss.is_finite() || rss < 0.0 || n == 0 || n_params == 0 {
        return f64::NAN;
    }
    let rss = rss.max(f64::MIN_POSITIVE);
    let n = n as f64;
    let k = n_params as f64;
    n * (rss / n).ln() + k * n.ln()
}

/// Nested **likelihood-ratio χ²** for two Gaussian linear models on the same `n`:
///
/// ```text
/// χ² = n · ln(RSS_null / RSS_alt)
/// ```
///
/// `null` is the **restricted** model (fewer parameters), `alt` the fuller
/// nested model. Under H₀ (extra coefficients are zero), asymptotically
/// `χ² ~ χ²(df)` with `df = k_alt − k_null`.
///
/// Returns `f64::NAN` if RSS values are invalid or `RSS_null < RSS_alt`
/// (numerical noise can make this slightly negative — clamped to 0 when
/// within a tiny relative tolerance).
pub fn nested_lr_chi2(rss_null: f64, rss_alt: f64, n: usize) -> f64 {
    if n == 0 || !rss_null.is_finite() || !rss_alt.is_finite() || rss_null <= 0.0 || rss_alt <= 0.0
    {
        return f64::NAN;
    }
    let ratio = rss_null / rss_alt;
    if ratio < 1.0 {
        // Allow tiny numerical inversion
        if (rss_alt - rss_null) / rss_alt.max(1.0) < 1e-12 {
            return 0.0;
        }
        return f64::NAN;
    }
    (n as f64) * ratio.ln()
}

/// Nested **F** statistic for linear models:
///
/// ```text
/// F = [(RSS_null − RSS_alt) / Δk] / [RSS_alt / (n − k_alt)]
/// ```
///
/// with `Δk = k_alt − k_null > 0` and `n > k_alt`.
pub fn nested_f_stat(rss_null: f64, rss_alt: f64, n: usize, k_null: usize, k_alt: usize) -> f64 {
    if k_alt <= k_null || n <= k_alt {
        return f64::NAN;
    }
    if !rss_null.is_finite() || !rss_alt.is_finite() || rss_alt <= 0.0 {
        return f64::NAN;
    }
    let dk = (k_alt - k_null) as f64;
    let num = (rss_null - rss_alt) / dk;
    let den = rss_alt / (n - k_alt) as f64;
    if den <= 0.0 || num < 0.0 {
        if num.abs() < 1e-12 * rss_alt.max(1.0) {
            return 0.0;
        }
        return f64::NAN;
    }
    num / den
}

/// Upper-tail χ² survival function `P(X > x)` for `X ~ χ²(df)`.
///
/// Uses the regularized gamma `Q(df/2, x/2)`. Adequate for model-comparison
/// reporting (not a high-precision special-function library).
pub fn chi2_sf(x: f64, df: f64) -> f64 {
    if !x.is_finite() || !df.is_finite() || df <= 0.0 {
        return f64::NAN;
    }
    if x <= 0.0 {
        return 1.0;
    }
    // Q(a,z) = Γ(a,z)/Γ(a)  with a = df/2, z = x/2
    gamma_q(df * 0.5, x * 0.5)
}

/// Bundle of comparison scores for one fitted model on a fixed dataset.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelFitScores {
    /// Sample size.
    pub n: usize,
    /// Number of free parameters (incl. intercept).
    pub n_params: usize,
    /// Residual sum of squares.
    pub rss: f64,
    /// R² (optional; `NaN` if not provided).
    pub r2: f64,
    /// Adjusted R².
    pub adj_r2: f64,
    /// Akaike information criterion (Gaussian form).
    pub aic: f64,
    /// Bayesian information criterion (Gaussian form).
    pub bic: f64,
}

impl ModelFitScores {
    /// Build scores from residuals summary.
    pub fn from_rss(n: usize, n_params: usize, rss: f64, r2: f64) -> Self {
        Self {
            n,
            n_params,
            rss,
            r2,
            adj_r2: adjusted_r2(r2, n, n_params),
            aic: aic_gaussian(rss, n, n_params),
            bic: bic_gaussian(rss, n, n_params),
        }
    }

    /// Build from actual / predicted vectors.
    pub fn from_predictions(actual: &[f64], predicted: &[f64], n_params: usize, r2: f64) -> Self {
        let n = actual.len();
        let rss_v = rss(actual, predicted);
        Self::from_rss(n, n_params, rss_v, r2)
    }

    /// Convenience: MSE·n = RSS when lengths match.
    pub fn from_mse(n: usize, n_params: usize, mse_val: f64, r2: f64) -> Self {
        let rss_v = if mse_val.is_finite() && n > 0 {
            mse_val * n as f64
        } else {
            f64::NAN
        };
        Self::from_rss(n, n_params, rss_v, r2)
    }
}

/// Nested comparison of a restricted model vs a fuller nested alternative.
#[derive(Debug, Clone, PartialEq)]
pub struct NestedModelTest {
    /// LR χ² = `n · ln(RSS_null / RSS_alt)`.
    pub lr_chi2: f64,
    /// Degrees of freedom `k_alt − k_null`.
    pub df: usize,
    /// Approximate upper-tail p-value `P(χ²_df > lr_chi2)`.
    pub lr_p: f64,
    /// Nested F statistic.
    pub f_stat: f64,
    /// Scores for the null (restricted) model.
    pub null: ModelFitScores,
    /// Scores for the alternative (fuller) model.
    pub alt: ModelFitScores,
}

impl NestedModelTest {
    /// Compare nested Gaussian linear models via RSS and parameter counts.
    pub fn from_rss(
        n: usize,
        rss_null: f64,
        k_null: usize,
        r2_null: f64,
        rss_alt: f64,
        k_alt: usize,
        r2_alt: f64,
    ) -> Self {
        let df = k_alt.saturating_sub(k_null);
        let lr = nested_lr_chi2(rss_null, rss_alt, n);
        let lr_p = if df > 0 && lr.is_finite() {
            chi2_sf(lr, df as f64)
        } else {
            f64::NAN
        };
        let f = nested_f_stat(rss_null, rss_alt, n, k_null, k_alt);
        Self {
            lr_chi2: lr,
            df,
            lr_p,
            f_stat: f,
            null: ModelFitScores::from_rss(n, k_null, rss_null, r2_null),
            alt: ModelFitScores::from_rss(n, k_alt, rss_alt, r2_alt),
        }
    }
}

// --- incomplete gamma (upper) for chi2_sf ------------------------------------

/// Regularized upper incomplete gamma `Q(a,x) = Γ(a,x)/Γ(a)`.
fn gamma_q(a: f64, x: f64) -> f64 {
    if a <= 0.0 || x < 0.0 || !a.is_finite() || !x.is_finite() {
        return f64::NAN;
    }
    if x == 0.0 {
        return 1.0;
    }
    // For large x relative to a, series for P and Q=1-P; use continued fraction for Q when x > a+1
    if x < a + 1.0 {
        // Series for lower P, then Q = 1 - P
        let p = gamma_p_series(a, x);
        (1.0 - p).clamp(0.0, 1.0)
    } else {
        gamma_q_cont_frac(a, x).clamp(0.0, 1.0)
    }
}

fn log_gamma(a: f64) -> f64 {
    // Lanczos approximation (g=7)
    const COEFF: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_654_078_915e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if a < 0.5 {
        // reflection
        let pi = std::f64::consts::PI;
        return pi.ln() - (pi * a).sin().ln() - log_gamma(1.0 - a);
    }
    let z = a - 1.0;
    let mut x = COEFF[0];
    for (i, &c) in COEFF.iter().enumerate().skip(1) {
        x += c / (z + i as f64);
    }
    let t = z + 7.5;
    (2.0 * std::f64::consts::PI).sqrt().ln() + (z + 0.5) * t.ln() - t + x.ln()
}

fn gamma_p_series(a: f64, x: f64) -> f64 {
    // P(a,x) = x^a e^{-x} / Γ(a) · Σ x^n / (a(a+1)…(a+n))
    let mut sum = 1.0 / a;
    let mut term = sum;
    for n in 1..200 {
        term *= x / (a + n as f64);
        sum += term;
        if term.abs() < sum.abs() * 1e-14 {
            break;
        }
    }
    (-x + a * x.ln() - log_gamma(a)).exp() * sum
}

fn gamma_q_cont_frac(a: f64, x: f64) -> f64 {
    // Lentz continued fraction for Q
    const FPMIN: f64 = 1e-300;
    let mut b = x + 1.0 - a;
    let mut c = 1.0 / FPMIN;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..200 {
        let an = -i as f64 * (i as f64 - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = b + an / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < 1e-14 {
            break;
        }
    }
    (-x + a * x.ln() - log_gamma(a)).exp() * h
}

// Silence unused if mse not needed elsewhere in this module — used in tests.
#[allow(dead_code)]
fn _mse_bridge(a: &[f64], p: &[f64]) -> f64 {
    mse(a, p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rss_and_aic_prefer_simpler_when_equal_fit() {
        let y = [1.0, 2.0, 3.0, 4.0];
        let pred = [1.0, 2.0, 3.0, 4.0];
        let r = rss(&y, &pred);
        assert!(r < 1e-15);
        // Perfect fit: more params → worse AIC
        let aic1 = aic_gaussian(1e-20, 4, 2);
        let aic2 = aic_gaussian(1e-20, 4, 4);
        assert!(aic1 < aic2);
    }

    #[test]
    fn nested_lr_zero_when_rss_equal() {
        let chi = nested_lr_chi2(10.0, 10.0, 100);
        assert!((chi - 0.0).abs() < 1e-12);
    }

    #[test]
    fn nested_lr_positive_when_alt_better() {
        let chi = nested_lr_chi2(20.0, 10.0, 50);
        assert!(chi > 0.0);
        // n * ln(2) ≈ 34.66
        assert!((chi - 50.0 * 2.0_f64.ln()).abs() < 1e-9);
    }

    #[test]
    fn chi2_sf_reasonable() {
        // χ²(1) mean is 1; P(X>1) ≈ 0.317
        let p = chi2_sf(1.0, 1.0);
        assert!(p > 0.25 && p < 0.40, "p={p}");
        // large x → small p
        assert!(chi2_sf(20.0, 1.0) < 1e-4);
    }

    #[test]
    fn adj_r2_penalizes() {
        let a = adjusted_r2(0.9, 100, 10);
        assert!(a < 0.9 && a > 0.8);
    }
}
