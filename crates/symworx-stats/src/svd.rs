// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Singular Value Decomposition (SVD)
//!
//! Wrapper around `ndarray_linalg` (LAPACK).

use ndarray::{Array1, Array2};
use ndarray_linalg::SVD;

/// Result of Singular Value Decomposition: A = U Σ Vᵀ
#[derive(Debug, Clone)]
pub struct Svd {
    /// ( U \in \mathbb{R}^{m \times m} \) contains the left singular vectors (orthogonal)
    pub u: Array2<f64>,
    /// \( \Sigma \in \mathbb{R}^{m \times n} \) is a diagonal matrix of singular values
    pub s: Array1<f64>,
    /// \( V^T \in \mathbb{R}^{n \times n} \) contains the right singular vectors (orthogonal)
    pub vt: Array2<f64>,
}

impl Svd {
    /// Compute the thin SVD of matrix `A`.
    pub fn compute(a: &Array2<f64>) -> Self {
        let (u_opt, s, vt_opt) = a
            .svd(true, true)
            .expect("SVD computation failed. Matrix may be singular or ill-conditioned.");

        let u = u_opt.expect("U matrix was not computed");
        let vt = vt_opt.expect("VT matrix was not computed");

        Self { u, s, vt }
    }

    /// Returns the rank of the matrix (number of significant singular values).
    pub fn rank(&self, tol: Option<f64>) -> usize {
        let tol = tol.unwrap_or(1e-10);
        self.s.iter().filter(|&&val| val > tol).count()
    }

    /// Explained variance ratio (useful for PCA).
    pub fn explained_variance_ratio(&self) -> Array1<f64> {
        let total: f64 = self.s.mapv(|x| x.powi(2)).sum();
        if total < 1e-12 {
            Array1::zeros(self.s.len())
        } else {
            self.s.mapv(|x| x.powi(2) / total)
        }
    }
}

// TESTS
#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    #[test]
    fn test_svd_basic() {
        let a = array![[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]];

        let svd = Svd::compute(&a);
        assert_eq!(svd.s.len(), 2);
        assert!(svd.s[0] >= svd.s[1]);
    }

    #[test]
    fn test_svd_reconstruction() {
        let a = array![[1.0, 2.0], [3.0, 4.0],];

        let svd = Svd::compute(&a);
        let sigma = Array2::from_diag(&svd.s);
        let reconstructed = svd.u.dot(&sigma).dot(&svd.vt);

        let max_error = (&a - &reconstructed)
            .mapv(f64::abs)
            .into_iter()
            .fold(0.0_f64, |acc, v| acc.max(v));
        assert!(max_error < 1e-8, "Reconstruction error too large");
    }
}
