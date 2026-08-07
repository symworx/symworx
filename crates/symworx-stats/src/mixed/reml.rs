// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Profiled REML/ML internals for single-factor random effects.
//!
//! Supports random intercept (`q = 1`) and multi-column designs such as
//! intercept + slope (`q = 2`) with diagonal or unstructured relative
//! covariance `Γ` (`G = σ² Γ`). Uses the Woodbury / block form of
//! `V = I + Z (I ⊗ Γ) Z'` so the dense `n × n` covariance is never formed.

use std::collections::HashMap;

use ndarray::{
    Array1,
    Array2,
};
use ndarray_linalg::cholesky::{
    DeterminantC,
    InverseC,
    SolveC,
};

use super::types::{
    CovStructure,
    EstimationMethod,
    MixedError,
};

/// Per-group random design after remapping labels to `0..g-1`.
pub struct GroupedRandomDesign {
    pub labels: Vec<usize>,
    pub q: usize,
    /// Observation row indices for each group.
    pub rows: Vec<Vec<usize>>,
    /// `Z_j` (`n_j × q`) for each group.
    pub z_blocks: Vec<Array2<f64>>,
}

pub struct ProfileFit {
    pub beta: Array1<f64>,
    pub sigma2: f64,
    /// BLUPs: `n_groups × q`.
    pub blups: Array2<f64>,
    pub loglik: f64,
    /// Relative RE covariance `Γ` (`G = σ² Γ`).
    pub gamma: Array2<f64>,
}

pub fn augment_x(x: &Array2<f64>, fit_intercept: bool) -> Array2<f64> {
    if !fit_intercept {
        return x.to_owned();
    }
    let n = x.nrows();
    let p = x.ncols();
    let mut x_aug = Array2::<f64>::ones((n, p + 1));
    if p > 0 {
        x_aug.slice_mut(ndarray::s![.., 1..]).assign(x);
    }
    x_aug
}

/// Remap labels and build per-group `Z` blocks from a full `n × q` design.
pub fn build_grouped_design(groups: &[usize], z_full: &Array2<f64>) -> Result<GroupedRandomDesign, MixedError> {
    if groups.is_empty() {
        return Err(MixedError::InvalidGroups {
            detail: "empty groups".into(),
        });
    }
    if z_full.nrows() != groups.len() {
        return Err(MixedError::LengthMismatch {
            what: "z_cols.nrows vs groups".into(),
            expected: groups.len(),
            got: z_full.nrows(),
        });
    }
    let q = z_full.ncols();
    if q == 0 {
        return Err(MixedError::InvalidGroups {
            detail: "z_cols must have at least one column".into(),
        });
    }

    let mut label_to_idx: HashMap<usize, usize> = HashMap::new();
    let mut labels: Vec<usize> = Vec::new();
    let mut rows: Vec<Vec<usize>> = Vec::new();
    for (i, &g) in groups.iter().enumerate() {
        let idx = *label_to_idx.entry(g).or_insert_with(|| {
            let id = labels.len();
            labels.push(g);
            rows.push(Vec::new());
            id
        });
        rows[idx].push(i);
    }

    let mut z_blocks = Vec::with_capacity(rows.len());
    for r in &rows {
        let nj = r.len();
        let mut zj = Array2::<f64>::zeros((nj, q));
        for (local, &row) in r.iter().enumerate() {
            for c in 0..q {
                zj[[local, c]] = z_full[[row, c]];
            }
        }
        z_blocks.push(zj);
    }

    Ok(GroupedRandomDesign {
        labels,
        q,
        rows,
        z_blocks,
    })
}

/// Number of unconstrained variance parameters for `Γ`.
pub fn n_theta_params(q: usize, structure: CovStructure) -> usize {
    match structure {
        CovStructure::Diagonal => q,
        CovStructure::Unstructured => q * (q + 1) / 2,
    }
}

/// Build unconstrained θ with log-diagonal levels `log_diag0` (first RE) and
/// `log_diag_rest` (remaining diagonals); off-diagonals (unstructured) are 0.
pub fn theta_start_scale(q: usize, structure: CovStructure, log_diag0: f64, log_diag_rest: f64) -> Array1<f64> {
    match structure {
        CovStructure::Diagonal => {
            let mut t = Array1::<f64>::zeros(q);
            for i in 0..q {
                t[i] = if i == 0 { log_diag0 } else { log_diag_rest };
            }
            t
        }
        CovStructure::Unstructured => {
            let mut t = Array1::<f64>::zeros(n_theta_params(q, structure));
            let mut k = 0;
            for j in 0..q {
                for i in j..q {
                    if i == j {
                        t[k] = if j == 0 { log_diag0 } else { log_diag_rest };
                    } else {
                        t[k] = 0.0;
                    }
                    k += 1;
                }
            }
            t
        }
    }
}

/// Several unconstrained starts for multi-parameter variance search.
pub fn theta_multistarts(q: usize, structure: CovStructure) -> Vec<Array1<f64>> {
    vec![
        theta_start_scale(q, structure, 0.0, -1.0),  // Γ diag ~ (1, 0.37, …)
        theta_start_scale(q, structure, 1.0, 0.0),   // larger RE
        theta_start_scale(q, structure, -1.0, -2.0), // smaller RE
        theta_start_scale(q, structure, 0.5, -0.5),
        theta_start_scale(q, structure, -0.5, -1.5),
    ]
}

/// Map unconstrained `θ` → SPD relative covariance `Γ` (`q × q`).
pub fn gamma_from_theta(theta: &Array1<f64>, q: usize, structure: CovStructure) -> Result<Array2<f64>, MixedError> {
    let need = n_theta_params(q, structure);
    if theta.len() != need {
        return Err(MixedError::InvalidGroups {
            detail: format!("theta length {} != expected {need} for q={q}", theta.len()),
        });
    }
    match structure {
        CovStructure::Diagonal => {
            let mut g = Array2::<f64>::zeros((q, q));
            for i in 0..q {
                g[[i, i]] = theta[i].exp().max(1e-12);
            }
            Ok(g)
        }
        CovStructure::Unstructured => {
            // Log-Cholesky: lower L with positive diagonal, Γ = L Lᵀ
            let mut l = Array2::<f64>::zeros((q, q));
            let mut k = 0;
            for j in 0..q {
                for i in j..q {
                    if i == j {
                        l[[i, j]] = theta[k].exp().max(1e-8);
                    } else {
                        l[[i, j]] = theta[k];
                    }
                    k += 1;
                }
            }
            Ok(l.dot(&l.t()))
        }
    }
}

/// Tiny relative covariance used as the “no RE / OLS boundary” candidate.
pub fn near_zero_gamma(q: usize) -> Array2<f64> {
    Array2::from_diag(&Array1::from_elem(q, 1e-12))
}

fn invert_spd(a: &Array2<f64>) -> Result<Array2<f64>, MixedError> {
    a.invc()
        .map_err(|_| MixedError::SingularSystem { stage: "gamma_inverse" })
}

fn log_det_spd(a: &Array2<f64>) -> Result<f64, MixedError> {
    a.detc()
        .map(|d| if d > 0.0 { d.ln() } else { f64::NEG_INFINITY })
        .map_err(|_| MixedError::SingularSystem { stage: "log_det" })
}

fn solvec_spd(a: &Array2<f64>, b: &Array1<f64>) -> Result<Array1<f64>, MixedError> {
    a.solvec(b)
        .map_err(|_| MixedError::SingularSystem { stage: "spd_solve" })
}

/// Apply `V⁻¹` for `V = I + Z (I ⊗ Γ) Z'`.
fn apply_vinv(
    v: &Array1<f64>,
    design: &GroupedRandomDesign,
    gamma_inv: &Array2<f64>,
) -> Result<Array1<f64>, MixedError> {
    let q = design.q;
    let mut out = v.to_owned();

    for (j, rows) in design.rows.iter().enumerate() {
        let zj = &design.z_blocks[j];
        // Z_j' v_j
        let mut ztv = Array1::<f64>::zeros(q);
        for c in 0..q {
            let mut s = 0.0;
            for (local, &row) in rows.iter().enumerate() {
                s += zj[[local, c]] * v[row];
            }
            ztv[c] = s;
        }
        // M_j = Γ⁻¹ + Z_j' Z_j
        let sj = zj.t().dot(zj);
        let mj = gamma_inv + &sj;
        let w = solvec_spd(&mj, &ztv)?;
        // out_j -= Z_j w
        for (local, &row) in rows.iter().enumerate() {
            let mut zw = 0.0;
            for c in 0..q {
                zw += zj[[local, c]] * w[c];
            }
            out[row] -= zw;
        }
    }
    Ok(out)
}

fn log_det_v(design: &GroupedRandomDesign, gamma: &Array2<f64>, gamma_inv: &Array2<f64>) -> Result<f64, MixedError> {
    let g = design.z_blocks.len() as f64;
    let mut acc = g * log_det_spd(gamma)?;
    for zj in &design.z_blocks {
        let sj = zj.t().dot(zj);
        let mj = gamma_inv + &sj;
        acc += log_det_spd(&mj)?;
    }
    Ok(acc)
}

fn gls_beta(
    y: &Array1<f64>,
    x_aug: &Array2<f64>,
    design: &GroupedRandomDesign,
    gamma_inv: &Array2<f64>,
) -> Result<(Array1<f64>, Array2<f64>), MixedError> {
    let p = x_aug.ncols();
    let vinv_y = apply_vinv(y, design, gamma_inv)?;

    let mut xtvx = Array2::<f64>::zeros((p, p));
    let mut xtvy = Array1::<f64>::zeros(p);
    let mut vinv_x_cols: Vec<Array1<f64>> = Vec::with_capacity(p);

    for j in 0..p {
        let col = x_aug.column(j).to_owned();
        let vinv_col = apply_vinv(&col, design, gamma_inv)?;
        xtvy[j] = x_aug.column(j).dot(&vinv_y);
        vinv_x_cols.push(vinv_col);
    }
    for j in 0..p {
        for k in 0..=j {
            let v = x_aug.column(j).dot(&vinv_x_cols[k]);
            xtvx[[j, k]] = v;
            xtvx[[k, j]] = v;
        }
    }

    let beta = xtvx
        .solvec(&xtvy)
        .map_err(|_| MixedError::SingularSystem { stage: "fixed_effects" })?;
    Ok((beta, xtvx))
}

/// Profiled objective to **minimize** (≈ `-2 loglik` without shared constants).
pub fn profile_objective(
    y: &Array1<f64>,
    x_aug: &Array2<f64>,
    design: &GroupedRandomDesign,
    gamma: &Array2<f64>,
    method: EstimationMethod,
) -> Result<f64, MixedError> {
    let n = y.len();
    let p = x_aug.ncols();
    let gamma_inv = invert_spd(gamma)?;
    let (beta, xtvx) = gls_beta(y, x_aug, design, &gamma_inv)?;
    let fitted = x_aug.dot(&beta);
    let r = y - &fitted;
    let vinv_r = apply_vinv(&r, design, &gamma_inv)?;
    let quad = r.dot(&vinv_r);
    if !(quad.is_finite() && quad >= 0.0) {
        return Err(MixedError::SingularSystem { stage: "quadratic" });
    }

    let logdet_v = log_det_v(design, gamma, &gamma_inv)?;
    if !logdet_v.is_finite() {
        return Err(MixedError::SingularSystem { stage: "log_det_v" });
    }

    match method {
        EstimationMethod::Reml => {
            let df = (n - p) as f64;
            let sigma2 = (quad / df).max(f64::MIN_POSITIVE);
            let logdet_xtvx = log_det_spd(&xtvx)?;
            Ok(df * sigma2.ln() + logdet_v + logdet_xtvx + df)
        }
        EstimationMethod::Ml => {
            let nf = n as f64;
            let sigma2 = (quad / nf).max(f64::MIN_POSITIVE);
            Ok(nf * sigma2.ln() + logdet_v + nf)
        }
    }
}

pub fn profile_fit(
    y: &Array1<f64>,
    x_aug: &Array2<f64>,
    design: &GroupedRandomDesign,
    gamma: &Array2<f64>,
    method: EstimationMethod,
) -> Result<ProfileFit, MixedError> {
    let n = y.len();
    let p = x_aug.ncols();
    let q = design.q;
    let g = design.z_blocks.len();
    let gamma_inv = invert_spd(gamma)?;
    let (beta, xtvx) = gls_beta(y, x_aug, design, &gamma_inv)?;
    let fitted = x_aug.dot(&beta);
    let r = y - &fitted;
    let vinv_r = apply_vinv(&r, design, &gamma_inv)?;
    let quad = r.dot(&vinv_r).max(0.0);

    let (sigma2, loglik) = match method {
        EstimationMethod::Reml => {
            let df = (n - p) as f64;
            let sigma2 = (quad / df).max(f64::MIN_POSITIVE);
            let logdet_v = log_det_v(design, gamma, &gamma_inv)?;
            let logdet_xtvx = log_det_spd(&xtvx)?;
            let loglik =
                -0.5 * (df * (2.0 * std::f64::consts::PI).ln() + df * sigma2.ln() + logdet_v + logdet_xtvx + df);
            (sigma2, loglik)
        }
        EstimationMethod::Ml => {
            let nf = n as f64;
            let sigma2 = (quad / nf).max(f64::MIN_POSITIVE);
            let logdet_v = log_det_v(design, gamma, &gamma_inv)?;
            let loglik = -0.5 * (nf * (2.0 * std::f64::consts::PI).ln() + nf * sigma2.ln() + logdet_v + nf);
            (sigma2, loglik)
        }
    };

    // BLUP û_j = M_j⁻¹ Z_j' r_j with M_j = Γ⁻¹ + Z_j' Z_j
    // (Woodbury intermediate; equals G Z' V⁻¹ r under V = σ²(I + Z(I⊗Γ)Z')).
    let mut blups = Array2::<f64>::zeros((g, q));
    for (j, rows) in design.rows.iter().enumerate() {
        let zj = &design.z_blocks[j];
        let mut ztr = Array1::<f64>::zeros(q);
        for c in 0..q {
            let mut s = 0.0;
            for (local, &row) in rows.iter().enumerate() {
                s += zj[[local, c]] * r[row];
            }
            ztr[c] = s;
        }
        let sj = zj.t().dot(zj);
        let mj = &gamma_inv + &sj;
        let uj = solvec_spd(&mj, &ztr)?;
        for c in 0..q {
            blups[[j, c]] = uj[c];
        }
    }

    Ok(ProfileFit {
        beta,
        sigma2,
        blups,
        loglik,
        gamma: gamma.to_owned(),
    })
}

/// 1-D golden section (used for scalar `q = 1` log-variance).
pub fn golden_section_minimize<F>(f: F, lo: f64, hi: f64, tol: f64, max_iter: usize) -> (f64, f64, usize, bool)
where
    F: Fn(f64) -> f64,
{
    let gr = (5.0_f64.sqrt() - 1.0) / 2.0;
    let mut a = lo;
    let mut b = hi;
    let mut c = b - gr * (b - a);
    let mut d = a + gr * (b - a);
    let mut fc = f(c);
    let mut fd = f(d);
    let mut iterations = 0usize;
    let mut converged = false;

    for _ in 0..max_iter {
        iterations += 1;
        if (b - a).abs() < tol {
            converged = true;
            break;
        }
        if fc < fd {
            b = d;
            d = c;
            fd = fc;
            c = b - gr * (b - a);
            fc = f(c);
        } else {
            a = c;
            c = d;
            fc = fd;
            d = a + gr * (b - a);
            fd = f(d);
        }
    }

    let (x, fx) = if fc < fd { (c, fc) } else { (d, fd) };
    let mid = 0.5 * (a + b);
    let fm = f(mid);
    if fm < fx {
        (mid, fm, iterations, converged)
    } else {
        (x, fx, iterations, converged)
    }
}
