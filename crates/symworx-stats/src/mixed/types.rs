// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Public types for linear mixed models.

use std::{
    collections::HashMap,
    fmt,
};

use ndarray::{
    Array1,
    Array2,
};

use crate::linreg::LinearModel;

/// REML vs full maximum likelihood for variance components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EstimationMethod {
    /// Restricted maximum likelihood (default; less bias in σ²).
    #[default]
    Reml,
    /// Full maximum likelihood.
    Ml,
}

/// Covariance structure for a random-effect block `G` (`q × q`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CovStructure {
    /// Independent random effects (diagonal `G`).
    #[default]
    Diagonal,
    /// Full symmetric positive-definite `G` (e.g. intercept–slope correlation).
    Unstructured,
}

/// Options for [`super::lmer`].
#[derive(Debug, Clone, PartialEq)]
pub struct LmerConfig {
    /// REML (default) or ML.
    pub method: EstimationMethod,
    /// Maximum iterations per variance-component search (each multi-start).
    pub max_iter: usize,
    /// Convergence tolerance (1-D golden section on `log θ`, or GD grad/param tol).
    pub tol: f64,
    /// If `true` (default), add an intercept column (same convention as OLS).
    pub fit_intercept: bool,
    /// Learning rate for multi-parameter GD (ignored for scalar random intercept).
    pub learning_rate: f64,
    /// Use Armijo line search in multi-parameter GD (recommended).
    pub line_search: bool,
    /// Number of random-effect parameter multi-starts when `q > 1`
    /// (clamped to available built-in starts). Ignored for random intercept.
    pub n_restarts: usize,
}

impl Default for LmerConfig {
    fn default() -> Self {
        Self {
            method: EstimationMethod::Reml,
            max_iter: 400,
            tol: 1e-8,
            fit_intercept: true,
            learning_rate: 0.05,
            line_search: true,
            n_restarts: 4,
        }
    }
}

/// One random-effect term (grouping factor).
///
/// - `z_cols = None` → random intercept (`q = 1`, column of ones).
/// - `z_cols = Some(Z)` with `Z` shape `n × q` → general random effects
///   (e.g. columns `[1, t]` for linear growth).
#[derive(Debug, Clone)]
pub struct RandomTerm {
    /// Factor name (key into [`MixedModel::blups`] / [`MixedModel::re_cov`]).
    pub name: String,
    /// Group label per observation (length `n`). Arbitrary `usize` labels OK.
    pub groups: Array1<usize>,
    /// Optional random-effect design columns. `None` ⇒ random intercept.
    pub z_cols: Option<Array2<f64>>,
    /// Covariance structure for this term's `G` block.
    pub cov_structure: CovStructure,
}

impl RandomTerm {
    /// Random intercept for `name` with per-observation group labels.
    pub fn random_intercept(name: impl Into<String>, groups: Array1<usize>) -> Self {
        Self {
            name: name.into(),
            groups,
            z_cols: None,
            cov_structure: CovStructure::Diagonal,
        }
    }

    /// Linear growth RE: columns `[1, time]` with unstructured `2 × 2` `G`.
    ///
    /// `time` must have the same length as `groups`. Uses
    /// [`super::design::z_intercept_slope`].
    pub fn linear_growth(
        name: impl Into<String>,
        groups: Array1<usize>,
        time: &Array1<f64>,
    ) -> Result<Self, MixedError> {
        if time.len() != groups.len() {
            return Err(MixedError::LengthMismatch {
                what: "time vs groups".into(),
                expected: groups.len(),
                got: time.len(),
            });
        }
        Ok(Self {
            name: name.into(),
            groups,
            z_cols: Some(super::design::z_intercept_slope(time)),
            cov_structure: CovStructure::Unstructured,
        })
    }

    /// Same as [`Self::linear_growth`] but with **diagonal** `G` (independent
    /// intercept and slope variances; no intercept–slope covariance).
    pub fn linear_growth_diagonal(
        name: impl Into<String>,
        groups: Array1<usize>,
        time: &Array1<f64>,
    ) -> Result<Self, MixedError> {
        let mut term = Self::linear_growth(name, groups, time)?;
        term.cov_structure = CovStructure::Diagonal;
        Ok(term)
    }
}

/// Fitted linear mixed model (single grouping factor).
#[derive(Debug, Clone)]
pub struct MixedModel {
    /// Fixed-effect coefficients (population mean structure).
    pub fixed: LinearModel,
    /// Residual variance `σ²`.
    pub sigma2: f64,
    /// Unconstrained variance-component parameters (log-Cholesky / log-diag of `Γ`).
    pub theta: Array1<f64>,
    /// Named random-effect covariance matrices `G = σ² Γ` (`q × q`).
    pub re_cov: HashMap<String, Array2<f64>>,
    /// BLUPs per factor: shape `n_groups × q`, aligned with [`Self::group_labels`].
    pub blups: HashMap<String, Array2<f64>>,
    /// Random-effect dimension `q` per factor.
    pub re_dim: HashMap<String, usize>,
    /// Original group labels in BLUP row order.
    pub group_labels: HashMap<String, Vec<usize>>,
    /// Profiled log-likelihood (REML or ML according to [`Self::method`]).
    pub loglik: f64,
    /// Estimation method used.
    pub method: EstimationMethod,
    /// Number of observations.
    pub n: usize,
    /// Number of fixed-effect parameters (including intercept if fitted).
    pub n_fixed: usize,
    /// Number of groups per factor name.
    pub n_groups: HashMap<String, usize>,
    /// Whether the outer θ search met [`LmerConfig::tol`].
    pub converged: bool,
    /// Outer-search iterations performed.
    pub iterations: usize,
}

impl MixedModel {
    /// Population-level prediction (`random effects = 0`): `ŷ = X β`.
    pub fn predict(&self, x: &Array2<f64>) -> Array1<f64> {
        self.fixed.predict(x)
    }

    /// Subject-specific prediction: `ŷ = X β + Z û`.
    ///
    /// * `groups` — factor name → per-row group labels (same coding as fit).
    /// * `z_cols` — factor name → per-row RE design (`n × q`). For a pure
    ///   random intercept (`q = 1`), the entry may be omitted and a column of
    ///   ones is assumed.
    ///
    /// Unknown group labels contribute `0` for that unit.
    pub fn predict_conditional(
        &self,
        x: &Array2<f64>,
        groups: &HashMap<String, &[usize]>,
        z_cols: &HashMap<String, &Array2<f64>>,
    ) -> Result<Array1<f64>, MixedError> {
        let n = x.nrows();
        let mut yhat = self.fixed.predict(x);

        for (name, blups) in &self.blups {
            let q = *self.re_dim.get(name).unwrap_or(&blups.ncols());
            let labels = self
                .group_labels
                .get(name)
                .ok_or_else(|| MixedError::MissingFactor { name: name.clone() })?;
            let g_obs = groups
                .get(name.as_str())
                .copied()
                .ok_or_else(|| MixedError::MissingFactor { name: name.clone() })?;
            if g_obs.len() != n {
                return Err(MixedError::LengthMismatch {
                    what: format!("groups[{name}]"),
                    expected: n,
                    got: g_obs.len(),
                });
            }

            let z_owned: Option<Array2<f64>>;
            let z: &Array2<f64> = if let Some(zref) = z_cols.get(name.as_str()).copied() {
                if zref.nrows() != n {
                    return Err(MixedError::LengthMismatch {
                        what: format!("z_cols[{name}].nrows"),
                        expected: n,
                        got: zref.nrows(),
                    });
                }
                if zref.ncols() != q {
                    return Err(MixedError::LengthMismatch {
                        what: format!("z_cols[{name}].ncols"),
                        expected: q,
                        got: zref.ncols(),
                    });
                }
                zref
            } else if q == 1 {
                z_owned = Some(Array2::ones((n, 1)));
                z_owned.as_ref().unwrap()
            } else {
                return Err(MixedError::MissingFactor {
                    name: format!("z_cols[{name}] (required for q={q})"),
                });
            };

            let mut index: HashMap<usize, usize> = HashMap::with_capacity(labels.len());
            for (i, &lab) in labels.iter().enumerate() {
                index.insert(lab, i);
            }
            for (row, &lab) in g_obs.iter().enumerate() {
                if let Some(&j) = index.get(&lab) {
                    let mut s = 0.0;
                    for c in 0..q {
                        s += z[[row, c]] * blups[[j, c]];
                    }
                    yhat[row] += s;
                }
            }
        }
        Ok(yhat)
    }

    /// BLUPs for a named random factor (`n_groups × q`), if present.
    pub fn ranef(&self, name: &str) -> Option<&Array2<f64>> {
        self.blups.get(name)
    }

    /// `G[0, 0]` for `name` (intercept variance when first RE column is intercept).
    pub fn sigma_u2(&self, name: &str) -> Option<f64> {
        self.re_cov.get(name).map(|g| g[[0, 0]])
    }

    /// Full random-effect covariance `G` for `name`.
    pub fn re_covariance(&self, name: &str) -> Option<&Array2<f64>> {
        self.re_cov.get(name)
    }

    /// Short text summary of fixed effects and variance components.
    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!(
            "Linear mixed model ({:?})\n  n = {}, n_fixed = {}, converged = {}, iters = {}\n",
            self.method, self.n, self.n_fixed, self.converged, self.iterations
        ));
        s.push_str(&format!("  intercept = {:.6}\n", self.fixed.intercept));
        if self.fixed.coefficients.is_empty() {
            s.push_str("  coefficients = []\n");
        } else {
            let coefs: Vec<String> = self.fixed.coefficients.iter().map(|c| format!("{c:.6}")).collect();
            s.push_str(&format!("  coefficients = [{}]\n", coefs.join(", ")));
        }
        s.push_str(&format!("  sigma2 = {:.6}, loglik = {:.4}\n", self.sigma2, self.loglik));
        for (name, g) in &self.re_cov {
            let ng = self.n_groups.get(name).copied().unwrap_or(0);
            let q = self.re_dim.get(name).copied().unwrap_or(g.nrows());
            s.push_str(&format!("  re[{name}]: q = {q}, n_groups = {ng}, G =\n{g:.6}\n"));
        }
        s
    }
}

/// Errors from mixed-model fitting and prediction.
#[derive(Debug, Clone, PartialEq)]
pub enum MixedError {
    /// Empty `y` or `x`.
    EmptyData,
    /// Length / shape disagreement.
    LengthMismatch {
        /// What failed the check.
        what: String,
        /// Expected length / size.
        expected: usize,
        /// Actual length / size.
        got: usize,
    },
    /// No random terms, or empty group vector.
    InvalidGroups {
        /// Human-readable detail.
        detail: String,
    },
    /// Feature not supported yet (e.g. multiple grouping factors).
    UnsupportedStructure {
        /// Human-readable detail.
        detail: String,
    },
    /// Singular / non-SPD system during solve.
    SingularSystem {
        /// Stage name (`fixed_effects`, …).
        stage: &'static str,
    },
    /// Required factor name missing from maps.
    MissingFactor {
        /// Factor name.
        name: String,
    },
    /// Optimizer / search failed in a hard way (rare; soft non-convergence
    /// returns a model with `converged = false` instead).
    NonConvergence {
        /// Iterations performed.
        iterations: usize,
    },
}

impl fmt::Display for MixedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyData => write!(f, "empty data"),
            Self::LengthMismatch { what, expected, got } => {
                write!(f, "length mismatch for {what}: expected {expected}, got {got}")
            }
            Self::InvalidGroups { detail } => write!(f, "invalid groups: {detail}"),
            Self::UnsupportedStructure { detail } => {
                write!(f, "unsupported random structure: {detail}")
            }
            Self::SingularSystem { stage } => write!(f, "singular system at stage '{stage}'"),
            Self::MissingFactor { name } => write!(f, "missing factor '{name}'"),
            Self::NonConvergence { iterations } => {
                write!(f, "failed to converge after {iterations} iterations")
            }
        }
    }
}

impl std::error::Error for MixedError {}
