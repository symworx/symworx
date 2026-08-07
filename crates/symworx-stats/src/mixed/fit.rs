// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Public fit entry points for linear mixed models.

use std::collections::HashMap;

use ndarray::{
    Array1,
    Array2,
};
use symworx_math::optimize::{
    GradientDescentConfig,
    gradient_descent_fd,
};

use super::{
    reml::{
        augment_x,
        build_grouped_design,
        gamma_from_theta,
        golden_section_minimize,
        n_theta_params,
        near_zero_gamma,
        profile_fit,
        profile_objective,
        theta_multistarts,
    },
    types::{
        CovStructure,
        LmerConfig,
        MixedError,
        MixedModel,
        RandomTerm,
    },
};
use crate::linreg::LinearModel;

/// Fit a linear mixed model with default [`LmerConfig`].
pub fn lmer_default(y: &Array1<f64>, x: &Array2<f64>, random: &[RandomTerm]) -> Result<MixedModel, MixedError> {
    lmer(y, x, random, &LmerConfig::default())
}

/// Fit a Gaussian linear mixed model (single grouping factor).
///
/// # Arguments
/// * `y` — response (`n`)
/// * `x` — fixed design without intercept when `config.fit_intercept` (`n × p`)
/// * `random` — exactly one [`RandomTerm`] (`z_cols = None` or `n × q` design)
/// * `config` — REML/ML and optimizer knobs
///
/// # Random structures
/// * Random intercept: [`RandomTerm::random_intercept`]
/// * Linear growth: [`RandomTerm::linear_growth`] → `Z = [1, t]`, unstructured `G`
/// * Diagonal growth: [`RandomTerm::linear_growth_diagonal`]
pub fn lmer(
    y: &Array1<f64>,
    x: &Array2<f64>,
    random: &[RandomTerm],
    config: &LmerConfig,
) -> Result<MixedModel, MixedError> {
    let n = y.len();
    if n == 0 || x.nrows() == 0 {
        return Err(MixedError::EmptyData);
    }
    if x.nrows() != n {
        return Err(MixedError::LengthMismatch {
            what: "x.nrows vs y".into(),
            expected: n,
            got: x.nrows(),
        });
    }
    if random.len() != 1 {
        return Err(MixedError::UnsupportedStructure {
            detail: format!("single grouping factor only (got {} terms)", random.len()),
        });
    }
    let term = &random[0];
    if term.groups.len() != n {
        return Err(MixedError::LengthMismatch {
            what: format!("random[{}].groups", term.name),
            expected: n,
            got: term.groups.len(),
        });
    }

    let z_full = match &term.z_cols {
        None => Array2::ones((n, 1)),
        Some(z) => {
            if z.nrows() != n {
                return Err(MixedError::LengthMismatch {
                    what: format!("random[{}].z_cols.nrows", term.name),
                    expected: n,
                    got: z.nrows(),
                });
            }
            if z.ncols() == 0 {
                return Err(MixedError::InvalidGroups {
                    detail: "z_cols must have at least one column".into(),
                });
            }
            z.to_owned()
        }
    };

    let group_slice = term.groups.as_slice().ok_or_else(|| MixedError::InvalidGroups {
        detail: "groups must be contiguous".into(),
    })?;
    let design = build_grouped_design(group_slice, &z_full)?;
    let n_groups = design.labels.len();
    let q = design.q;

    let x_aug = augment_x(x, config.fit_intercept);
    let n_fixed = x_aug.ncols();
    if n_fixed == 0 {
        return Err(MixedError::InvalidGroups {
            detail: "fixed design has zero columns (enable fit_intercept or pass features)".into(),
        });
    }
    if n <= n_fixed {
        return Err(MixedError::InvalidGroups {
            detail: format!("need n > n_fixed (n={n}, n_fixed={n_fixed})"),
        });
    }

    let structure = if q == 1 {
        CovStructure::Diagonal
    } else {
        term.cov_structure
    };

    let (theta_hat, gamma, iterations, converged) = optimize_variance(y, &x_aug, &design, structure, config)?;

    let fit = profile_fit(y, &x_aug, &design, &gamma, config.method)?;
    let g_mat = &fit.gamma * fit.sigma2;

    let fixed = if config.fit_intercept {
        LinearModel::from_packed(&fit.beta)
    } else {
        LinearModel {
            intercept: 0.0,
            coefficients: fit.beta.clone(),
        }
    };

    let mut re_cov = HashMap::new();
    re_cov.insert(term.name.clone(), g_mat);

    let mut blups = HashMap::new();
    blups.insert(term.name.clone(), fit.blups);

    let mut re_dim = HashMap::new();
    re_dim.insert(term.name.clone(), q);

    let mut group_labels = HashMap::new();
    group_labels.insert(term.name.clone(), design.labels.clone());

    let mut n_groups_map = HashMap::new();
    n_groups_map.insert(term.name.clone(), n_groups);

    Ok(MixedModel {
        fixed,
        sigma2: fit.sigma2,
        theta: theta_hat,
        re_cov,
        blups,
        re_dim,
        group_labels,
        loglik: fit.loglik,
        method: config.method,
        n,
        n_fixed,
        n_groups: n_groups_map,
        converged,
        iterations,
    })
}

fn optimize_variance(
    y: &Array1<f64>,
    x_aug: &Array2<f64>,
    design: &super::reml::GroupedRandomDesign,
    structure: CovStructure,
    config: &LmerConfig,
) -> Result<(Array1<f64>, Array2<f64>, usize, bool), MixedError> {
    let q = design.q;
    let method = config.method;

    let gamma0 = near_zero_gamma(q);
    let obj0 = profile_objective(y, x_aug, design, &gamma0, method)?;

    if q == 1 {
        let obj = |log_theta: f64| -> f64 {
            let g = Array2::from_elem((1, 1), log_theta.exp().max(1e-12));
            profile_objective(y, x_aug, design, &g, method).unwrap_or(f64::INFINITY)
        };
        let (log_hat, obj_hat, iters, search_ok) =
            golden_section_minimize(obj, -12.0, 8.0, config.tol, config.max_iter);

        if obj0 <= obj_hat {
            return Ok((Array1::from(vec![-20.0]), gamma0, iters, true));
        }
        let theta = Array1::from(vec![log_hat]);
        let gamma = gamma_from_theta(&theta, 1, CovStructure::Diagonal)?;
        return Ok((theta, gamma, iters, search_ok));
    }

    // Multi-parameter: multi-start FD gradient descent
    let starts = theta_multistarts(q, structure);
    let n_starts = config.n_restarts.clamp(1, starts.len());

    let gd_cfg = GradientDescentConfig {
        learning_rate: config.learning_rate,
        max_iter: config.max_iter,
        grad_tol: config.tol,
        param_tol: config.tol,
        line_search: config.line_search,
        ..GradientDescentConfig::default()
    };

    let loss = |th: &Array1<f64>| -> f64 {
        match gamma_from_theta(th, q, structure) {
            Ok(g) => profile_objective(y, x_aug, design, &g, method).unwrap_or(f64::INFINITY),
            Err(_) => f64::INFINITY,
        }
    };

    let mut best_theta: Option<Array1<f64>> = None;
    let mut best_obj = f64::INFINITY;
    let mut best_iters = 0usize;
    let mut best_converged = false;
    let mut total_iters = 0usize;

    for start in starts.into_iter().take(n_starts) {
        let opt = gradient_descent_fd(loss, start, &gd_cfg);
        total_iters += opt.iterations;
        if opt.loss.is_finite() && opt.loss < best_obj {
            best_obj = opt.loss;
            best_theta = Some(opt.params);
            best_iters = opt.iterations;
            best_converged = opt.converged;
        }
    }

    // Compare to near-zero RE boundary
    if obj0 <= best_obj {
        let mut th = Array1::<f64>::from_elem(n_theta_params(q, structure), -20.0);
        if structure == CovStructure::Unstructured {
            let mut k = 0;
            for j in 0..q {
                for i in j..q {
                    th[k] = if i == j { -20.0 } else { 0.0 };
                    k += 1;
                }
            }
        }
        return Ok((th, gamma0, total_iters.max(best_iters), true));
    }

    let theta_hat = best_theta.ok_or(MixedError::NonConvergence {
        iterations: total_iters,
    })?;

    if theta_hat.len() != n_theta_params(q, structure) {
        return Err(MixedError::NonConvergence {
            iterations: total_iters,
        });
    }

    if !best_obj.is_finite() {
        return Err(MixedError::NonConvergence {
            iterations: total_iters,
        });
    }

    let mut converged = best_converged;
    let gamma = match gamma_from_theta(&theta_hat, q, structure) {
        Ok(g) => g,
        Err(_) => {
            converged = false;
            near_zero_gamma(q)
        }
    };

    Ok((theta_hat, gamma, total_iters, converged))
}
