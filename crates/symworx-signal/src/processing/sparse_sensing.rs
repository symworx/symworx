// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Sparse sensing and compressed sensing recovery.
//!
//! Classical tools from data-driven science and engineering (Brunton & Kutz):
//! undersampled linear measurements `y = Φ x` (or `y = Φ Ψ s` in a sparsity
//! basis) and recovery of a sparse coefficient vector via
//!
//! - **ISTA** — iterative soft-thresholding (proximal gradient for ℓ1)
//! - **OMP** — orthogonal matching pursuit (greedy support selection)
//!
//! Soft-thresholding reuses [`symworx_stats::soft_threshold`].

use ndarray::{
    Array1,
    Array2,
};
use ndarray_linalg::LeastSquaresSvd;
use symworx_stats::soft_threshold;

/// Result of a sparse recovery algorithm.
#[derive(Debug, Clone)]
pub struct SparseRecoveryResult {
    /// Recovered coefficient / signal vector (length = n columns of sensing matrix).
    pub coefficients: Array1<f64>,
    /// `‖y − Θ x̂‖₂` at termination.
    pub residual_norm: f64,
    /// Iterations performed (ISTA steps, or OMP atoms selected).
    pub iterations: usize,
    /// Number of non-zero coefficients (`|x̂ᵢ| > tol`).
    pub sparsity: usize,
}

/// Configuration for [`ista`].
#[derive(Debug, Clone)]
pub struct IstaConfig {
    /// ℓ1 penalty weight λ ≥ 0.
    pub lambda: f64,
    /// Maximum iterations.
    pub max_iter: usize,
    /// Stop when `‖x^{k+1} − x^k‖₂ < tol`.
    pub tol: f64,
    /// Optional fixed step size. If `None`, uses `1 / ‖Θ‖₂²` estimate
    /// (power iteration on `ΘᵀΘ`).
    pub step_size: Option<f64>,
    /// Threshold for counting non-zeros in the result.
    pub sparsity_tol: f64,
}

impl Default for IstaConfig {
    fn default() -> Self {
        Self {
            lambda: 0.1,
            max_iter: 500,
            tol: 1e-8,
            step_size: None,
            sparsity_tol: 1e-8,
        }
    }
}

/// Linear measurement `y = Φ x`.
pub fn measure(phi: &Array2<f64>, x: &Array1<f64>) -> Array1<f64> {
    assert_eq!(
        phi.ncols(),
        x.len(),
        "Φ has {} columns but x has length {}",
        phi.ncols(),
        x.len()
    );
    phi.dot(x)
}

/// Build the effective sensing matrix `Θ = Φ Ψ` (composition of measurement
/// and sparsity basis). If `psi` is `None`, uses the identity (`Θ = Φ`).
pub fn effective_sensing(phi: &Array2<f64>, psi: Option<&Array2<f64>>) -> Array2<f64> {
    match psi {
        None => phi.to_owned(),
        Some(psi) => {
            assert_eq!(
                phi.ncols(),
                psi.nrows(),
                "Φ cols ({}) must match Ψ rows ({})",
                phi.ncols(),
                psi.nrows()
            );
            phi.dot(psi)
        }
    }
}

/// Reconstruct the signal in the ambient domain: `x = Ψ s` (or `x = s` if
/// `psi` is `None`).
pub fn reconstruct_signal(psi: Option<&Array2<f64>>, coeffs: &Array1<f64>) -> Array1<f64> {
    match psi {
        None => coeffs.to_owned(),
        Some(psi) => {
            assert_eq!(
                psi.ncols(),
                coeffs.len(),
                "Ψ cols ({}) must match coefficient length ({})",
                psi.ncols(),
                coeffs.len()
            );
            psi.dot(coeffs)
        }
    }
}

/// Iterative Soft-Thresholding Algorithm (ISTA) for
/// `min_x  ½ ‖y − Θ x‖₂² + λ ‖x‖₁`.
///
/// # Arguments
/// * `theta` — sensing / dictionary matrix (m × n), often `Φ` or `ΦΨ`
/// * `y` — measurements (length m)
/// * `config` — λ, iterations, step size
pub fn ista(theta: &Array2<f64>, y: &Array1<f64>, config: &IstaConfig) -> SparseRecoveryResult {
    let m = theta.nrows();
    let n = theta.ncols();
    assert_eq!(y.len(), m, "y length must equal number of rows of Θ");
    assert!(config.lambda >= 0.0, "lambda must be non-negative");

    let step = config
        .step_size
        .unwrap_or_else(|| 1.0 / (lipschitz_estimate(theta) + 1e-12));

    let mut x = Array1::<f64>::zeros(n);
    let mut iterations = 0;

    for iter in 0..config.max_iter {
        iterations = iter + 1;
        // gradient of ½‖y − Θx‖² is −Θᵀ(y − Θx)
        let residual = y - &theta.dot(&x);
        let grad = theta.t().dot(&residual);
        let z = &x + &(&grad * step);
        let threshold = config.lambda * step;

        let mut x_new = Array1::zeros(n);
        for i in 0..n {
            x_new[i] = soft_threshold(z[i], threshold);
        }

        let delta = (&x_new - &x).mapv(|v| v * v).sum().sqrt();
        x = x_new;
        if delta < config.tol {
            break;
        }
    }

    let residual_norm = (y - &theta.dot(&x)).mapv(|v| v * v).sum().sqrt();
    let sparsity = x.iter().filter(|&&v| v.abs() > config.sparsity_tol).count();

    SparseRecoveryResult {
        coefficients: x,
        residual_norm,
        iterations,
        sparsity,
    }
}

/// Orthogonal Matching Pursuit: greedily select up to `sparsity` columns of
/// `Θ` to approximate `y` in least-squares sense.
///
/// # Arguments
/// * `theta` — dictionary / sensing matrix (m × n)
/// * `y` — measurements
/// * `sparsity` — maximum number of non-zero coefficients (atoms)
/// * `tol` — stop early if residual norm drops below this
pub fn omp(theta: &Array2<f64>, y: &Array1<f64>, sparsity: usize, tol: f64) -> SparseRecoveryResult {
    let m = theta.nrows();
    let n = theta.ncols();
    assert_eq!(y.len(), m);
    assert!(sparsity > 0, "sparsity must be positive");

    let max_atoms = sparsity.min(n).min(m);
    let mut residual = y.to_owned();
    let mut selected: Vec<usize> = Vec::with_capacity(max_atoms);
    let mut x = Array1::<f64>::zeros(n);

    for _ in 0..max_atoms {
        // Correlate residual with all columns; pick largest absolute correlation
        // among unused indices
        let mut best_j = 0usize;
        let mut best_corr = 0.0_f64;
        for j in 0..n {
            if selected.contains(&j) {
                continue;
            }
            let col = theta.column(j);
            let corr = col.dot(&residual).abs();
            if corr > best_corr {
                best_corr = corr;
                best_j = j;
            }
        }
        selected.push(best_j);

        // Least-squares on the selected support: min ‖y − Θ_S β‖
        let theta_s = columns_as_matrix(theta, &selected);
        let beta = match theta_s.least_squares(y) {
            Ok(sol) => sol.solution,
            Err(_) => {
                // Fallback: leave previous x
                break;
            }
        };

        x.fill(0.0);
        for (k, &j) in selected.iter().enumerate() {
            x[j] = beta[k];
        }

        residual = y - &theta.dot(&x);
        let rnorm = residual.dot(&residual).sqrt();
        if rnorm < tol {
            break;
        }
    }

    let residual_norm = residual.dot(&residual).sqrt();
    let sparsity_count = x.iter().filter(|&&v| v.abs() > 1e-12).count();

    SparseRecoveryResult {
        coefficients: x,
        residual_norm,
        iterations: selected.len(),
        sparsity: sparsity_count,
    }
}

/// Random Gaussian sensing matrix with unit-norm columns (deterministic seed).
///
/// Entries ~ N(0, 1/m), then each column is L2-normalized. Useful for
/// educational compressed-sensing demos.
pub fn random_gaussian_sensing(m: usize, n: usize, seed: u64) -> Array2<f64> {
    assert!(m > 0 && n > 0);
    let mut rng = seed;
    let mut phi = Array2::<f64>::zeros((m, n));
    let scale = 1.0 / (m as f64).sqrt();

    for j in 0..n {
        for i in 0..m {
            // Box-Muller
            let u1 = (lcg_next(&mut rng) as f64 / u64::MAX as f64).clamp(1e-12, 1.0 - 1e-12);
            let u2 = lcg_next(&mut rng) as f64 / u64::MAX as f64;
            let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            phi[[i, j]] = z * scale;
        }
        let col = phi.column(j);
        let norm = col.dot(&col).sqrt();
        if norm > 1e-15 {
            phi.column_mut(j).mapv_inplace(|v| v / norm);
        }
    }
    phi
}

/// Orthonormal DCT-II basis matrix (n × n): columns are DCT basis vectors.
///
/// Many natural signals are sparse (or compressible) in this basis.
pub fn dct_basis(n: usize) -> Array2<f64> {
    assert!(n > 0);
    let mut psi = Array2::<f64>::zeros((n, n));
    let scale0 = (1.0 / n as f64).sqrt();
    let scale = (2.0 / n as f64).sqrt();

    for k in 0..n {
        let s = if k == 0 { scale0 } else { scale };
        for i in 0..n {
            let angle = std::f64::consts::PI * k as f64 * (2.0 * i as f64 + 1.0) / (2.0 * n as f64);
            psi[[i, k]] = s * angle.cos();
        }
    }
    psi
}

/// Subsample rows of the identity (canonical “pixel” measurements).
///
/// `indices` selects which ambient coordinates are observed. Returns `Φ`
/// of shape `(indices.len(), n)`.
pub fn row_selection_sensing(n: usize, indices: &[usize]) -> Array2<f64> {
    let m = indices.len();
    let mut phi = Array2::<f64>::zeros((m, n));
    for (row, &idx) in indices.iter().enumerate() {
        assert!(idx < n, "index {idx} out of range for n={n}");
        phi[[row, idx]] = 1.0;
    }
    phi
}

/// Estimate Lipschitz constant of `∇(½‖y−Θx‖²)` ≈ largest eigenvalue of `ΘᵀΘ`
/// via a few power iterations.
fn lipschitz_estimate(theta: &Array2<f64>) -> f64 {
    let n = theta.ncols();
    if n == 0 {
        return 1.0;
    }
    let mut v = Array1::from_elem(n, 1.0 / (n as f64).sqrt());
    for _ in 0..20 {
        let w = theta.t().dot(&theta.dot(&v));
        let norm = w.dot(&w).sqrt();
        if norm < 1e-15 {
            return 1.0;
        }
        v = w / norm;
    }
    let w = theta.t().dot(&theta.dot(&v));
    v.dot(&w).abs().max(1e-12)
}

fn columns_as_matrix(theta: &Array2<f64>, indices: &[usize]) -> Array2<f64> {
    let m = theta.nrows();
    let mut out = Array2::zeros((m, indices.len()));
    for (k, &j) in indices.iter().enumerate() {
        out.column_mut(k).assign(&theta.column(j));
    }
    out
}

fn lcg_next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(1664525).wrapping_add(1013904223);
    *state
}

// ——— Tests ———

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    #[test]
    fn test_measure_identity() {
        let phi = Array2::eye(3);
        let x = array![1.0, 2.0, 3.0];
        let y = measure(&phi, &x);
        assert_eq!(y, x);
    }

    #[test]
    fn test_omp_exact_sparse() {
        // Dictionary: identity — recover sparse x from full measurements
        let n = 8;
        let theta = Array2::eye(n);
        let mut x_true = Array1::zeros(n);
        x_true[1] = 3.0;
        x_true[5] = -1.5;
        let y = measure(&theta, &x_true);

        let rec = omp(&theta, &y, 2, 1e-12);
        assert_eq!(rec.sparsity, 2);
        assert!((rec.coefficients[1] - 3.0).abs() < 1e-8);
        assert!((rec.coefficients[5] + 1.5).abs() < 1e-8);
        assert!(rec.residual_norm < 1e-10);
    }

    #[test]
    fn test_ista_sparse_recovery() {
        let n = 10;
        let theta = Array2::eye(n);
        let mut x_true = Array1::zeros(n);
        x_true[2] = 2.0;
        x_true[7] = -1.0;
        let y = measure(&theta, &x_true);

        let cfg = IstaConfig {
            lambda: 0.01,
            max_iter: 200,
            tol: 1e-10,
            step_size: Some(0.5),
            sparsity_tol: 1e-4,
        };
        let rec = ista(&theta, &y, &cfg);
        assert!((rec.coefficients[2] - 2.0).abs() < 0.05);
        assert!((rec.coefficients[7] + 1.0).abs() < 0.05);
        assert!(rec.sparsity <= 3);
    }

    #[test]
    fn test_compressed_sensing_gaussian_omp() {
        // Sparse x in canonical basis; m < n Gaussian measurements
        let n = 20;
        let m = 8;
        let mut x_true = Array1::zeros(n);
        x_true[3] = 1.5;
        x_true[11] = -2.0;
        x_true[17] = 0.8;

        let phi = random_gaussian_sensing(m, n, 42);
        let y = measure(&phi, &x_true);
        let rec = omp(&phi, &y, 3, 1e-8);

        // Support should include the large atoms
        assert!(rec.coefficients[3].abs() > 0.5, "coeff 3 = {}", rec.coefficients[3]);
        assert!(rec.coefficients[11].abs() > 0.5, "coeff 11 = {}", rec.coefficients[11]);
        let err = (&rec.coefficients - &x_true).mapv(|v| v * v).sum().sqrt();
        assert!(err < 0.5, "recovery L2 error = {err}");
    }

    #[test]
    fn test_dct_basis_orthonormal() {
        let n = 4;
        let psi = dct_basis(n);
        let gram = psi.t().dot(&psi);
        for i in 0..n {
            for j in 0..n {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((gram[[i, j]] - expected).abs() < 1e-10, "G[{i},{j}] = {}", gram[[i, j]]);
            }
        }
    }

    #[test]
    fn test_effective_sensing_and_reconstruct() {
        let phi = row_selection_sensing(4, &[0, 2]);
        let psi = Array2::eye(4);
        let theta = effective_sensing(&phi, Some(&psi));
        assert_eq!(theta.nrows(), 2);
        assert_eq!(theta.ncols(), 4);

        let s = array![1.0, 0.0, 2.0, 0.0];
        let x = reconstruct_signal(Some(&psi), &s);
        assert_eq!(x, s);
    }

    #[test]
    fn test_row_selection_sensing() {
        let phi = row_selection_sensing(5, &[1, 3]);
        assert_eq!(phi.shape(), &[2, 5]);
        assert!((phi[[0, 1]] - 1.0).abs() < 1e-15);
        assert!((phi[[1, 3]] - 1.0).abs() < 1e-15);
        assert!((phi[[0, 0]]).abs() < 1e-15);
    }
}
