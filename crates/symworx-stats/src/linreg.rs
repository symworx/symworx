// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Linear regression utilities.
//!
//! Includes Ordinary Least Squares (L2) and Lasso (L1) regression via coordinate descent.

use ndarray::{Array1, Array2, Axis, s};

#[cfg(feature = "linalg")]
use ndarray_linalg::Inverse;

/// Ordinary Least Squares (L2) linear regression with intercept.
///
/// This implementation uses LAPACK (via `ndarray-linalg`) for a fast and
/// numerically stable matrix inverse when the `linalg` feature is enabled.
///
/// When the `linalg` feature is **not** enabled, this function will panic
/// with a clear message. For dependency-minimal builds, enable the feature
/// only where regression is actually needed.
#[cfg(feature = "linalg")]
pub fn l2(x: &Array2<f64>, y: &Array1<f64>) -> Array1<f64> {
    let n_samples = x.nrows();
    let n_features = x.ncols();

    // Augment X with column of ones for intercept
    let mut x_aug = Array2::<f64>::ones((n_samples, n_features + 1));
    x_aug.slice_mut(s![.., 1..]).assign(x);

    let xtx = x_aug.t().dot(&x_aug);
    let _xty = x_aug.t().dot(y);

    let beta = xtx
        .inv()
        .expect("Matrix inversion failed — XᵀX is singular");

    // Return slopes only (exclude intercept)
    beta.slice(s![1.., ..]).to_owned().remove_axis(Axis(1))
}

/// Ordinary Least Squares (L2) linear regression with intercept (stub).
///
/// This version is active when the `linalg` feature is disabled.
/// It will panic at runtime with instructions to enable the feature.
#[cfg(not(feature = "linalg"))]
pub fn l2(_x: &Array2<f64>, _y: &Array1<f64>) -> Array1<f64> {
    panic!(
        "symworx_stats::l2 requires the `linalg` feature (which pulls ndarray-linalg + cauchy + LAPACK backend). \
         Enable it in Cargo.toml with features = [\"linalg\"] on the symworx-stats dependency if you need regression."
    )
}

/// Lasso regression (L1 regularized) using coordinate descent.
pub fn l1(
    x: &Array2<f64>,
    y: &Array1<f64>,
    alpha: f64, // regularization strength
    max_iter: usize,
    tol: f64,
) -> Array1<f64> {
    let n_samples = x.nrows();
    let n_features = x.ncols();

    // Center the data
    let x_mean = x.mean_axis(Axis(0)).unwrap();
    let y_mean = y.mean().unwrap();

    let x_centered = x - &x_mean;
    let y_centered = y - y_mean;

    let mut beta = Array1::<f64>::zeros(n_features);
    let mut beta_old = beta.clone();

    for _iter in 0..max_iter {
        beta_old.assign(&beta);

        for j in 0..n_features {
            // Compute partial residual excluding feature j
            let mut residual = y_centered.clone();
            for k in 0..n_features {
                if k != j {
                    let col = x_centered.column(k);
                    residual -= &(&col * beta[k]);
                }
            }

            let xj = x_centered.column(j);
            let xj_norm = xj.dot(&xj);

            if xj_norm < 1e-12 {
                beta[j] = 0.0;
                continue;
            }

            let rho = xj.dot(&residual);

            // Soft-thresholding — FIXED: cast n_samples to f64
            beta[j] = soft_threshold(rho, alpha * n_samples as f64) / xj_norm;
        }

        // Check convergence
        let diff = (&beta - &beta_old).mapv(|v| v.abs()).sum();
        if diff < tol {
            break;
        }
    }

    // Add intercept back
    let intercept = y_mean - x_mean.dot(&beta);
    let mut result = Array1::zeros(n_features + 1);
    result[0] = intercept;
    result.slice_mut(s![1..]).assign(&beta);

    result
}

/// Soft thresholding operator: S(x, λ) = sign(x) * max(|x| - λ, 0)
fn soft_threshold(x: f64, lambda: f64) -> f64 {
    if x > lambda {
        x - lambda
    } else if x < -lambda {
        x + lambda
    } else {
        0.0
    }
}

// TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    #[cfg(feature = "linalg")]
    #[test]
    fn test_l2_regression() {
        let x = array![[1.0], [2.0], [3.0], [4.0]];
        let y = array![2.0, 3.0, 5.0, 7.0];

        let coeffs = l2(&x, &y);
        assert_eq!(coeffs.len(), 1);
        assert!((coeffs[0] - 1.6).abs() < 0.2);
    }

    #[test]
    fn test_l1_basic() {
        let x = array![[1.0], [2.0], [3.0], [4.0], [5.0]];
        let y = array![2.2, 4.1, 5.9, 8.0, 10.1];

        let coeffs = l1(&x, &y, 0.1, 200, 1e-6);

        assert_eq!(coeffs.len(), 2); // intercept + 1 slope
        assert!(coeffs[1] > 1.0); // slope should be positive and reasonable
    }
}
