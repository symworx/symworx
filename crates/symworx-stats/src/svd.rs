// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Singular Value Decomposition (SVD)
//!
//! Wrapper around `ndarray_linalg` (LAPACK).

use ndarray::{
    Array1,
    Array2,
    s,
};
use ndarray_linalg::SVD;

/// Result of Singular Value Decomposition: A = U Σ Vᵀ
///
/// Used as the backbone for PCA, low-rank approximation, and (downstream)
/// data-driven dynamics methods such as DMD.
#[derive(Debug, Clone)]
pub struct Svd {
    /// Left singular vectors U (m × min(m, n) for thin SVD).
    pub u: Array2<f64>,
    /// Singular values Σ on the diagonal (length min(m, n), descending).
    pub s: Array1<f64>,
    /// Right singular vectors Vᵀ (min(m, n) × n for thin SVD).
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

    /// Number of singular values retained in this decomposition.
    pub fn n_singular(&self) -> usize {
        self.s.len()
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

    /// Truncate to the leading `k` singular components (rank-k factors).
    ///
    /// Returns `(U_k, s_k, Vᵀ_k)` with shapes `(m × k)`, `(k,)`, `(k × n)`.
    /// `k` is clamped to `min(k, n_singular)`.
    pub fn truncate(&self, k: usize) -> (Array2<f64>, Array1<f64>, Array2<f64>) {
        let k = k.min(self.s.len());
        let u_k = self.u.slice(s![.., 0..k]).to_owned();
        let s_k = self.s.slice(s![0..k]).to_owned();
        let vt_k = self.vt.slice(s![0..k, ..]).to_owned();
        (u_k, s_k, vt_k)
    }

    /// Rank-`k` reconstruction `A_k = U_k Σ_k Vᵀ_k`.
    ///
    /// This is the truncated SVD low-rank approximation used throughout
    /// data-driven science and engineering (Brunton & Kutz).
    pub fn reconstruct_rank_k(&self, k: usize) -> Array2<f64> {
        let (u_k, s_k, vt_k) = self.truncate(k);
        if k == 0 {
            return Array2::zeros((self.u.nrows(), self.vt.ncols()));
        }
        // U_k * diag(s_k) * Vᵀ_k
        let mut us = u_k;
        for j in 0..k {
            let sj = s_k[j];
            us.column_mut(j).mapv_inplace(|v| v * sj);
        }
        us.dot(&vt_k)
    }

    /// Full reconstruction from all retained singular components.
    pub fn reconstruct(&self) -> Array2<f64> {
        self.reconstruct_rank_k(self.s.len())
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
        let a = array![[1.0, 2.0], [3.0, 4.0]];

        let svd = Svd::compute(&a);
        let reconstructed = svd.reconstruct();

        let max_error = (&a - &reconstructed)
            .mapv(f64::abs)
            .into_iter()
            .fold(0.0_f64, |acc, v| acc.max(v));
        assert!(max_error < 1e-8, "Reconstruction error too large");
    }

    #[test]
    fn test_rank_k_low_rank_matrix() {
        // Rank-1 matrix: outer product of [1,2,3] and [1,0]
        let a = array![[1.0, 0.0], [2.0, 0.0], [3.0, 0.0]];
        let svd = Svd::compute(&a);

        assert_eq!(svd.rank(Some(1e-8)), 1);

        let a1 = svd.reconstruct_rank_k(1);
        let max_error = (&a - &a1)
            .mapv(f64::abs)
            .into_iter()
            .fold(0.0_f64, |acc, v| acc.max(v));
        assert!(max_error < 1e-8, "rank-1 reconstruction should be exact");

        let (u_k, s_k, vt_k) = svd.truncate(1);
        assert_eq!(u_k.ncols(), 1);
        assert_eq!(s_k.len(), 1);
        assert_eq!(vt_k.nrows(), 1);
    }

    #[test]
    fn test_truncate_clamps_k() {
        let a = array![[1.0, 2.0], [3.0, 4.0]];
        let svd = Svd::compute(&a);
        let (u_k, s_k, vt_k) = svd.truncate(100);
        assert_eq!(s_k.len(), svd.n_singular());
        assert_eq!(u_k.ncols(), svd.n_singular());
        assert_eq!(vt_k.nrows(), svd.n_singular());
    }
}
