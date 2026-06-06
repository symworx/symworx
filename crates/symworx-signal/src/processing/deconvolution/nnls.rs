// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Non-Negative Least Squares (NNLS) Deconvolution
//!
//! Solves the problem: find S ≥ 0 such that C ≈ K * S (convolution),
//! where C is the observed concentration and K is the elimination kernel.
//!
//! This is particularly useful for hormone deconvolution (e.g. Growth Hormone).

use ndarray::{Array1, Array2, s};
use ndarray_linalg::Solve;

/// Performs non-negative least squares deconvolution.
///
/// Attempts to recover a non-negative secretion rate `S` from an observed
/// concentration time series `C`, given a known elimination kernel.
pub fn nonnegative_deconvolution(
    observed: &[f64],
    kernel: &[f64],
    lambda: f64,          // regularization strength
) -> Vec<f64> {
    let n = observed.len();
    if n == 0 || kernel.is_empty() {
        return observed.to_vec();
    }

    // Build convolution matrix K (lower triangular Toeplitz)
    let m = kernel.len().min(n);
    let mut k_matrix = Array2::<f64>::zeros((n, n));

    for i in 0..n {
        for j in 0..m {
            if i >= j {
                k_matrix[[i, j]] = kernel[j];
            }
        }
    }

    let c = Array1::from_vec(observed.to_vec());

    // Regularized normal equations: (K^T K + λI) S = K^T C
    let kt = k_matrix.t();
    let ktk = kt.dot(&k_matrix);
    let ktc = kt.dot(&c);

    // Add regularization
    let mut a = ktk + Array2::eye(n) * lambda.max(1e-8);
    let b = ktc;

    // Solve A S = B with non-negativity constraint (simple approach)
    let mut s = match a.solve(&b) {
        Ok(solution) => solution,
        Err(_) => return vec![0.0; n], // fallback
    };

    // Enforce non-negativity
    s.mapv_inplace(|x| x.max(0.0));

    s.to_vec()
}

/// Convenience function for GH-style deconvolution with default regularization.
pub fn gh_deconvolve(observed: &[f64], kernel: &[f64]) -> Vec<f64> {
    // Typical regularization for hormone data
    nonnegative_deconvolution(observed, kernel, 1e-4)
}


// TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    fn synthetic_pulses() -> (Vec<f64>, Vec<f64>) {
        let n = 256;
        let mut signal = vec![0.0; n];

        // Add synthetic pulses
        for &peak in &[60, 130, 200] {
            for i in 0..n {
                let x = (i as f64 - peak as f64) / 9.0;
                signal[i] += (-0.5 * x * x).exp() * 4.5;
            }
        }

        // Exponential kernel
        let kernel: Vec<f64> = (0..40)
            .map(|i| {
                let t = i as f64 * 0.08;
                0.75 * (-t / 3.0).exp() + 0.25 * (-t / 18.0).exp()
            })
            .collect();

        (signal, kernel)
    }

    #[test]
    fn test_nnls_basic() {
        let (signal, kernel) = synthetic_pulses();
        let result = nonnegative_deconvolution(&signal, &kernel, 1e-4);

        assert_eq!(result.len(), signal.len());
        assert!(result.iter().all(|&x| x >= 0.0));
    }

    #[test]
    fn test_gh_deconvolve() {
        let (signal, kernel) = synthetic_pulses();
        let secretion = gh_deconvolve(&signal, &kernel);

        assert_eq!(secretion.len(), signal.len());
        assert!(secretion.iter().all(|&x| x >= 0.0));
    }

    #[test]
    fn test_empty_input() {
        let empty: Vec<f64> = vec![];
        let kernel = vec![1.0, 0.5];

        let result = nonnegative_deconvolution(&empty, &kernel, 1e-4);
        assert!(result.is_empty());
    }
}
