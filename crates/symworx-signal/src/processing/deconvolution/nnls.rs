// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Non-negative least squares deconvolution.
//!
//! Recovers a non-negative secretion series `S` such that `C ≈ K * S`, where
//! `C` is observed concentration and `K` is the causal convolution matrix of an
//! elimination kernel. Non-finite samples in `C` are dropped as missing rows.
//!
//! This is the hormone path (e.g. growth hormone on a regular sampling grid).

use ndarray::{
    Array1,
    Array2,
};
use ndarray_linalg::Solve;

use super::kernel::convolve;

/// Causal convolution design on the observed (finite) rows.
///
/// Returns `(K, c, n)` where `K` is `n_obs × n`, `c` is the finite samples,
/// and `n` is the full grid length (secretion length).
fn masked_system(observed: &[f64], kernel: &[f64]) -> Option<(Array2<f64>, Array1<f64>, usize)> {
    let n = observed.len();
    if n == 0 || kernel.is_empty() {
        return None;
    }
    let rows: Vec<usize> = (0..n).filter(|&i| observed[i].is_finite()).collect();
    if rows.is_empty() {
        return None;
    }
    let mut k_matrix = Array2::<f64>::zeros((rows.len(), n));
    for (r, &i) in rows.iter().enumerate() {
        let j_min = i.saturating_sub(kernel.len() - 1);
        for j in j_min..=i {
            k_matrix[[r, j]] = kernel[i - j];
        }
    }
    let c = Array1::from_iter(rows.iter().map(|&i| observed[i]));
    Some((k_matrix, c, n))
}

fn lipschitz_k(k: &Array2<f64>) -> f64 {
    let n = k.ncols();
    let mut v = Array1::from_elem(n, 1.0 / (n as f64).sqrt());
    for _ in 0..16 {
        let kv = k.dot(&v);
        let g = k.t().dot(&kv);
        let nrm = g.mapv(|x| x * x).sum().sqrt().max(1e-12);
        v = g / nrm;
    }
    let kv = k.dot(&v);
    kv.mapv(|x| x * x).sum().max(1e-12)
}

/// Regularized NNLS deconvolution of `observed` by `kernel`.
///
/// Solves `(KᵀK + λI) S = Kᵀ C` then clips `S` to be non-negative. `λ` is
/// `lambda.max(1e-8)`. Non-finite `observed` values are missing measurements;
/// secretion is still estimated on the full grid.
pub fn nonnegative_deconvolution(observed: &[f64], kernel: &[f64], lambda: f64) -> Vec<f64> {
    let n = observed.len();
    let Some((k_matrix, c, n_grid)) = masked_system(observed, kernel) else {
        return vec![0.0; n];
    };
    let kt = k_matrix.t();
    let ktk = kt.dot(&k_matrix);
    let ktc = kt.dot(&c);
    let a = ktk + Array2::<f64>::eye(n_grid) * lambda.max(1e-8);

    let mut s = match a.solve(&ktc) {
        Ok(solution) => solution,
        Err(_) => return vec![0.0; n_grid],
    };
    s.mapv_inplace(|x| x.max(0.0));
    s.to_vec()
}

/// Non-negative ISTA deconvolution with an ℓ1 penalty on secretion.
///
/// Minimizes `½ ‖C − K S‖₂² + λ ‖S‖₁` subject to `S ≥ 0`. The proximal map is
/// `max(z − λ step, 0)`. Non-finite samples in `C` are missing rows.
pub fn sparse_nonnegative_deconvolution(
    observed: &[f64],
    kernel: &[f64],
    lambda_l1: f64,
    max_iter: usize,
    tol: f64,
) -> Vec<f64> {
    let n = observed.len();
    let Some((k, c, n_grid)) = masked_system(observed, kernel) else {
        return vec![0.0; n];
    };
    let step = 1.0 / lipschitz_k(&k);
    let thresh = lambda_l1.max(0.0) * step;
    let mut s = Array1::<f64>::zeros(n_grid);
    for _ in 0..max_iter.max(1) {
        let residual = &c - &k.dot(&s);
        let z = &s + &(k.t().dot(&residual) * step);
        let mut s_new = Array1::<f64>::zeros(n_grid);
        for i in 0..n_grid {
            s_new[i] = (z[i] - thresh).max(0.0);
        }
        let delta = (&s_new - &s).mapv(|v| v * v).sum().sqrt();
        s = s_new;
        if delta < tol {
            break;
        }
    }
    s.to_vec()
}

/// GH-style deconvolution with default ridge (`λ = 1e-4`). Dense; not a burst train.
pub fn gh_deconvolve(observed: &[f64], kernel: &[f64]) -> Vec<f64> {
    nonnegative_deconvolution(observed, kernel, 1e-4)
}

/// Sparse GH deconvolution. `lambda_l1` is the ℓ1 weight on secretion.
pub fn gh_deconvolve_sparse(observed: &[f64], kernel: &[f64], lambda_l1: f64) -> Vec<f64> {
    sparse_nonnegative_deconvolution(observed, kernel, lambda_l1, 2000, 1e-8)
}

/// Strict interior local maxima of `s` at or above `floor`.
pub fn burst_indices(s: &[f64], floor: f64) -> Vec<usize> {
    let n = s.len();
    let mut out = Vec::new();
    if n < 3 {
        return out;
    }
    for i in 1..n - 1 {
        if s[i] >= floor && s[i] > s[i - 1] && s[i] > s[i + 1] {
            out.push(i);
        }
    }
    out
}

/// Non-negative least squares using only the listed secretion times.
pub fn nonnegative_deconvolution_on_support(observed: &[f64], kernel: &[f64], support: &[usize]) -> Vec<f64> {
    let n = observed.len();
    if support.is_empty() {
        return vec![0.0; n];
    }
    let Some((k_full, c, n_grid)) = masked_system(observed, kernel) else {
        return vec![0.0; n];
    };
    let p = support.len();
    let mut ks = Array2::<f64>::zeros((k_full.nrows(), p));
    for (j, &col) in support.iter().enumerate() {
        if col >= n_grid {
            return vec![0.0; n_grid];
        }
        for r in 0..k_full.nrows() {
            ks[[r, j]] = k_full[[r, col]];
        }
    }
    let ata = ks.t().dot(&ks) + Array2::<f64>::eye(p) * 1e-8;
    let atc = ks.t().dot(&c);
    let masses = match ata.solve(&atc) {
        Ok(v) => v,
        Err(_) => return vec![0.0; n_grid],
    };
    let mut s = vec![0.0; n_grid];
    for (j, &col) in support.iter().enumerate() {
        s[col] = masses[j].max(0.0);
    }
    s
}

/// Burst-train GH deconvolution: sparse ISTA, keep local maxima ≥ `floor`, debias.
pub fn gh_burst_deconvolve(observed: &[f64], kernel: &[f64], lambda_l1: f64, floor: f64) -> Vec<f64> {
    let draft = gh_deconvolve_sparse(observed, kernel, lambda_l1);
    let idx = burst_indices(&draft, floor);
    nonnegative_deconvolution_on_support(observed, kernel, &idx)
}

/// Reconstruct concentration by convolving secretion with the elimination kernel.
pub fn gh_reconvolve(secretion: &[f64], kernel: &[f64]) -> Vec<f64> {
    convolve(secretion, kernel)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::deconvolution::kernel::{
        convolve,
        exponential_kernel,
    };

    #[test]
    fn identity_kernel_recovers_a_nonnegative_series() {
        let observed = vec![0.0, 1.0, 0.5, 0.0];
        let kernel = vec![1.0];
        let s = nonnegative_deconvolution(&observed, &kernel, 1e-12);
        for (a, b) in s.iter().zip(observed.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn deconvolve_then_convolve_recovers_a_single_burst() {
        let mut secretion = vec![0.0; 20];
        secretion[5] = 2.0;
        let kernel = exponential_kernel(10.0, 15.0, 20);
        let observed = convolve(&secretion, &kernel);
        let recovered = gh_deconvolve(&observed, &kernel);
        let recon = gh_reconvolve(&recovered, &kernel);
        let err: f64 = observed
            .iter()
            .zip(recon.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            / observed.len() as f64;
        assert!(err.sqrt() < 0.05, "rmse {err}");
        assert!(recovered[5] > recovered[4] && recovered[5] > recovered[6]);
        assert!(recovered.iter().all(|&x| x >= 0.0));
    }

    #[test]
    fn missing_samples_are_skipped() {
        let kernel = vec![1.0, 0.5];
        let mut observed = convolve(&[0.0, 3.0, 0.0, 0.0], &kernel);
        observed[2] = f64::NAN;
        let s = nonnegative_deconvolution(&observed, &kernel, 1e-8);
        assert_eq!(s.len(), 4);
        assert!(s.iter().all(|&x| x >= 0.0 && x.is_finite()));
        assert!(s[1] > 1.0);
    }

    #[test]
    fn empty_input_returns_empty() {
        let result = nonnegative_deconvolution(&[], &[1.0, 0.5], 1e-4);
        assert!(result.is_empty());
    }

    #[test]
    fn sparse_deconv_keeps_two_separated_bursts() {
        let mut secretion = vec![0.0; 30];
        secretion[6] = 3.0;
        secretion[20] = 2.5;
        let kernel = exponential_kernel(10.0, 15.0, 30);
        let observed = convolve(&secretion, &kernel);
        let recovered = gh_deconvolve_sparse(&observed, &kernel, 0.05);
        assert!(recovered.iter().all(|&x| x >= 0.0));
        let n_nz = recovered.iter().filter(|&&x| x > 1e-6).count();
        assert!(n_nz < 12, "n_nz {n_nz}");
        assert!(recovered[6] > recovered[5] && recovered[6] > recovered[7]);
        assert!(recovered[20] > recovered[19] && recovered[20] > recovered[21]);
        let recon = gh_reconvolve(&recovered, &kernel);
        let err: f64 = observed
            .iter()
            .zip(recon.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            / observed.len() as f64;
        assert!(err.sqrt() < 0.4, "rmse {}", err.sqrt());
    }

    #[test]
    fn burst_deconv_returns_two_spikes() {
        let mut secretion = vec![0.0; 30];
        secretion[6] = 3.0;
        secretion[20] = 2.5;
        let kernel = exponential_kernel(10.0, 15.0, 30);
        let observed = convolve(&secretion, &kernel);
        let recovered = gh_burst_deconvolve(&observed, &kernel, 0.05, 0.5);
        let n_nz = recovered.iter().filter(|&&x| x > 1e-6).count();
        assert_eq!(n_nz, 2);
        assert!(recovered[6] > 1.0);
        assert!(recovered[20] > 1.0);
    }
}
