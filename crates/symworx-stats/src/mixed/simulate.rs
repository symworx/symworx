// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Synthetic longitudinal data for mixed-model tests and demos.

use ndarray::{
    Array1,
    Array2,
};
use rand::{
    Rng,
    SeedableRng,
    rngs::StdRng,
};

use super::types::MixedError;
use crate::linreg::LinearModel;

/// Known parameters for [`generate_random_intercept`].
#[derive(Debug, Clone)]
pub struct RandomInterceptSimSpec {
    /// Number of groups (subjects).
    pub n_groups: usize,
    /// Observations per group (balanced). Must be ≥ 1.
    pub n_per_group: usize,
    /// Fixed intercept.
    pub intercept: f64,
    /// Fixed slopes (length = number of columns in `x`).
    pub coefficients: Array1<f64>,
    /// Residual variance `σ² > 0`.
    pub sigma2: f64,
    /// Random-intercept variance `σ_u² ≥ 0`.
    pub sigma_u2: f64,
    /// RNG seed.
    pub seed: u64,
}

impl Default for RandomInterceptSimSpec {
    fn default() -> Self {
        Self {
            n_groups: 40,
            n_per_group: 5,
            intercept: 1.0,
            coefficients: Array1::from(vec![0.5]),
            sigma2: 1.0,
            sigma_u2: 2.0,
            seed: 42,
        }
    }
}

/// Simulated random-intercept dataset (long format).
#[derive(Debug, Clone)]
pub struct RandomInterceptData {
    /// Response.
    pub y: Array1<f64>,
    /// Fixed design **without** intercept column (`n × p`).
    pub x: Array2<f64>,
    /// Group label per row (`0 .. n_groups-1`).
    pub groups: Array1<usize>,
    /// True fixed model.
    pub true_fixed: LinearModel,
    /// True residual variance.
    pub true_sigma2: f64,
    /// True random-intercept variance.
    pub true_sigma_u2: f64,
    /// True random intercepts (length `n_groups`).
    pub true_u: Array1<f64>,
}

/// Generate balanced random-intercept data: `y = Xβ + u_g + ε`.
pub fn generate_random_intercept(spec: &RandomInterceptSimSpec) -> Result<RandomInterceptData, MixedError> {
    if spec.n_groups == 0 || spec.n_per_group == 0 {
        return Err(MixedError::InvalidGroups {
            detail: "n_groups and n_per_group must be positive".into(),
        });
    }
    if spec.sigma2 <= 0.0 || spec.sigma_u2 < 0.0 {
        return Err(MixedError::InvalidGroups {
            detail: "need sigma2 > 0 and sigma_u2 >= 0".into(),
        });
    }

    let p = spec.coefficients.len();
    let n = spec.n_groups * spec.n_per_group;
    let mut rng = StdRng::seed_from_u64(spec.seed);

    let mut x = Array2::<f64>::zeros((n, p));
    let mut groups = Array1::<usize>::zeros(n);
    let mut true_u = Array1::<f64>::zeros(spec.n_groups);
    let su = spec.sigma_u2.sqrt();
    let se = spec.sigma2.sqrt();

    for g in 0..spec.n_groups {
        true_u[g] = box_muller(&mut rng) * su;
        for k in 0..spec.n_per_group {
            let i = g * spec.n_per_group + k;
            groups[i] = g;
            for j in 0..p {
                x[[i, j]] = box_muller(&mut rng);
            }
        }
    }

    let true_fixed = LinearModel {
        intercept: spec.intercept,
        coefficients: spec.coefficients.clone(),
    };
    let mut y = true_fixed.predict(&x);
    for i in 0..n {
        y[i] += true_u[groups[i]] + box_muller(&mut rng) * se;
    }

    Ok(RandomInterceptData {
        y,
        x,
        groups,
        true_fixed,
        true_sigma2: spec.sigma2,
        true_sigma_u2: spec.sigma_u2,
        true_u,
    })
}

/// Known parameters for linear growth (random intercept + slope).
#[derive(Debug, Clone)]
pub struct LinearGrowthSimSpec {
    /// Number of subjects.
    pub n_groups: usize,
    /// Observations per subject when [`Self::n_per`] is `None` (balanced).
    pub n_per_group: usize,
    /// Optional unbalanced counts (length `n_groups`). Overrides `n_per_group`.
    pub n_per: Option<Vec<usize>>,
    /// Fixed intercept `β₀`.
    pub intercept: f64,
    /// Fixed slope of time `β₁`.
    pub slope: f64,
    /// Residual variance `σ² > 0`.
    pub sigma2: f64,
    /// Random-effect covariance `G` (`2 × 2`, SPD).
    pub re_cov: Array2<f64>,
    /// RNG seed.
    pub seed: u64,
}

impl Default for LinearGrowthSimSpec {
    fn default() -> Self {
        let mut g = Array2::<f64>::zeros((2, 2));
        g[[0, 0]] = 4.0;
        g[[1, 1]] = 0.25;
        g[[0, 1]] = 0.4;
        g[[1, 0]] = 0.4;
        Self {
            n_groups: 60,
            n_per_group: 6,
            n_per: None,
            intercept: 1.0,
            slope: 0.5,
            sigma2: 0.5,
            re_cov: g,
            seed: 11,
        }
    }
}

impl LinearGrowthSimSpec {
    /// Resolve per-group observation counts.
    pub fn counts(&self) -> Result<Vec<usize>, MixedError> {
        if let Some(ref v) = self.n_per {
            if v.len() != self.n_groups {
                return Err(MixedError::LengthMismatch {
                    what: "n_per vs n_groups".into(),
                    expected: self.n_groups,
                    got: v.len(),
                });
            }
            if v.contains(&0) {
                return Err(MixedError::InvalidGroups {
                    detail: "all n_per entries must be positive".into(),
                });
            }
            Ok(v.clone())
        } else {
            if self.n_groups == 0 || self.n_per_group == 0 {
                return Err(MixedError::InvalidGroups {
                    detail: "n_groups and n_per_group must be positive".into(),
                });
            }
            Ok(vec![self.n_per_group; self.n_groups])
        }
    }
}

/// Simulated linear-growth dataset.
#[derive(Debug, Clone)]
pub struct LinearGrowthData {
    /// Response.
    pub y: Array1<f64>,
    /// Fixed design without intercept: single column `time` (`n × 1`).
    pub x: Array2<f64>,
    /// Time covariate (same as `x` column 0).
    pub time: Array1<f64>,
    /// Group labels.
    pub groups: Array1<usize>,
    /// True fixed model (`intercept`, coefficient of time).
    pub true_fixed: LinearModel,
    /// True residual variance.
    pub true_sigma2: f64,
    /// True `G` (`2 × 2`).
    pub true_re_cov: Array2<f64>,
    /// True random effects `n_groups × 2` (intercept, slope).
    pub true_u: Array2<f64>,
}

/// Generate `y = β₀ + β₁ t + u0_g + u1_g · t + ε` with `(u0,u1) ~ N(0, G)`.
///
/// Time within each group is `0, 1, …, n_g−1` (unbalanced groups allowed via
/// [`LinearGrowthSimSpec::n_per`]).
pub fn generate_linear_growth(spec: &LinearGrowthSimSpec) -> Result<LinearGrowthData, MixedError> {
    if spec.sigma2 <= 0.0 {
        return Err(MixedError::InvalidGroups {
            detail: "need sigma2 > 0".into(),
        });
    }
    if spec.re_cov.nrows() != 2 || spec.re_cov.ncols() != 2 {
        return Err(MixedError::InvalidGroups {
            detail: "re_cov must be 2×2".into(),
        });
    }

    let counts = spec.counts()?;
    let n: usize = counts.iter().sum();
    let mut rng = StdRng::seed_from_u64(spec.seed);
    let chol = chol2(&spec.re_cov).ok_or(MixedError::SingularSystem {
        stage: "sim_re_cov_chol",
    })?;

    let mut time = Array1::<f64>::zeros(n);
    let mut groups = Array1::<usize>::zeros(n);
    let mut true_u = Array2::<f64>::zeros((spec.n_groups, 2));
    let se = spec.sigma2.sqrt();

    let mut row = 0usize;
    for g in 0..spec.n_groups {
        let z0 = box_muller(&mut rng);
        let z1 = box_muller(&mut rng);
        true_u[[g, 0]] = chol[[0, 0]] * z0;
        true_u[[g, 1]] = chol[[1, 0]] * z0 + chol[[1, 1]] * z1;

        let ng = counts[g];
        for k in 0..ng {
            let t = k as f64;
            time[row] = t;
            groups[row] = g;
            row += 1;
        }
    }

    let mut x = Array2::<f64>::zeros((n, 1));
    x.column_mut(0).assign(&time);

    let true_fixed = LinearModel {
        intercept: spec.intercept,
        coefficients: Array1::from(vec![spec.slope]),
    };

    let mut y = Array1::<f64>::zeros(n);
    for i in 0..n {
        let g = groups[i];
        let t = time[i];
        y[i] = spec.intercept + spec.slope * t + true_u[[g, 0]] + true_u[[g, 1]] * t + box_muller(&mut rng) * se;
    }

    Ok(LinearGrowthData {
        y,
        x,
        time,
        groups,
        true_fixed,
        true_sigma2: spec.sigma2,
        true_re_cov: spec.re_cov.clone(),
        true_u,
    })
}

/// Lower Cholesky factor of a 2×2 SPD matrix (returns `None` if not SPD).
fn chol2(a: &Array2<f64>) -> Option<Array2<f64>> {
    let a00 = a[[0, 0]];
    if a00 <= 0.0 {
        return None;
    }
    let l00 = a00.sqrt();
    let l10 = a[[1, 0]] / l00;
    let t = a[[1, 1]] - l10 * l10;
    if t <= 0.0 {
        return None;
    }
    let l11 = t.sqrt();
    let mut l = Array2::<f64>::zeros((2, 2));
    l[[0, 0]] = l00;
    l[[1, 0]] = l10;
    l[[1, 1]] = l11;
    Some(l)
}

fn box_muller(rng: &mut StdRng) -> f64 {
    let u1: f64 = rng.random::<f64>().clamp(f64::EPSILON, 1.0 - f64::EPSILON);
    let u2: f64 = rng.random::<f64>();
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}
