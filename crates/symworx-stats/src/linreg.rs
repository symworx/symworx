// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Linear regression utilities.
//!
//! Includes Ordinary Least Squares (L2), Ridge (L2-penalized), Lasso (L1),
//! and Elastic Net (mixed L1/L2) via closed-form solve or coordinate descent.
//!
//! These methods form the classical regression backbone used in data-driven
//! science and engineering (sparse regression, SINDy-style libraries, etc.).

use ndarray::{
    Array1,
    Array2,
    Axis,
    s,
};
#[cfg(feature = "linalg")]
use ndarray_linalg::Inverse;

/// Fitted linear model: intercept + coefficient vector.
///
/// Coefficients are stored without the intercept (length = number of features).
/// Prediction is `ŷ = intercept + X · coefficients`.
#[derive(Debug, Clone)]
pub struct LinearModel {
    /// Intercept term (bias).
    pub intercept: f64,
    /// Feature coefficients (length = n_features).
    pub coefficients: Array1<f64>,
}

impl LinearModel {
    /// Build from the legacy packed form `[intercept, β₀, β₁, …]`.
    pub fn from_packed(packed: &Array1<f64>) -> Self {
        assert!(!packed.is_empty(), "packed coefficient vector must not be empty");
        Self {
            intercept: packed[0],
            coefficients: packed.slice(s![1..]).to_owned(),
        }
    }

    /// Pack as `[intercept, β₀, β₁, …]` (legacy `l1` / `l2` layout).
    pub fn to_packed(&self) -> Array1<f64> {
        let mut out = Array1::zeros(self.coefficients.len() + 1);
        out[0] = self.intercept;
        out.slice_mut(s![1..]).assign(&self.coefficients);
        out
    }

    /// Predict responses for design matrix `x` (n_samples × n_features).
    pub fn predict(&self, x: &Array2<f64>) -> Array1<f64> {
        assert_eq!(
            x.ncols(),
            self.coefficients.len(),
            "feature dimension mismatch: X has {} cols, model has {} coefficients",
            x.ncols(),
            self.coefficients.len()
        );
        x.dot(&self.coefficients) + self.intercept
    }

    /// Number of features (excluding intercept).
    pub fn n_features(&self) -> usize {
        self.coefficients.len()
    }
}

/// Ordinary Least Squares (L2) linear regression with intercept.
///
/// Returns packed coefficients `[intercept, β…]` for backward compatibility.
/// Prefer [`ols`] for a structured [`LinearModel`].
///
/// This implementation uses LAPACK (via `ndarray-linalg`) for a fast and
/// numerically stable matrix inverse when the `linalg` feature is enabled.
///
/// When the `linalg` feature is **not** enabled, this function will panic
/// with a clear message. For dependency-minimal builds, enable the feature
/// only where regression is actually needed.
#[cfg(feature = "linalg")]
pub fn l2(x: &Array2<f64>, y: &Array1<f64>) -> Array1<f64> {
    ols(x, y).to_packed()
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

/// Ordinary Least Squares regression returning a structured [`LinearModel`].
#[cfg(feature = "linalg")]
pub fn ols(x: &Array2<f64>, y: &Array1<f64>) -> LinearModel {
    let n_samples = x.nrows();
    let n_features = x.ncols();
    assert_eq!(y.len(), n_samples, "X and y must have the same number of rows");

    // Augment X with column of ones for intercept
    let mut x_aug = Array2::<f64>::ones((n_samples, n_features + 1));
    x_aug.slice_mut(s![.., 1..]).assign(x);

    let xtx = x_aug.t().dot(&x_aug);
    let xty = x_aug.t().dot(y);

    let packed = xtx.inv().expect("Matrix inversion failed — XᵀX is singular").dot(&xty);
    LinearModel::from_packed(&packed)
}

/// Ordinary Least Squares (stub without `linalg`).
#[cfg(not(feature = "linalg"))]
pub fn ols(_x: &Array2<f64>, _y: &Array1<f64>) -> LinearModel {
    panic!(
        "symworx_stats::ols requires the `linalg` feature. \
         Enable it in Cargo.toml with features = [\"linalg\"] on the symworx-stats dependency."
    )
}

/// Ridge regression (L2-penalized least squares) with intercept.
///
/// Minimizes `‖y − Xβ − b‖² + α ‖β‖²` (intercept is not penalized).
/// Requires the `linalg` feature.
///
/// # Arguments
/// * `x` — design matrix (n_samples × n_features)
/// * `y` — response vector
/// * `alpha` — L2 regularization strength (α ≥ 0). `alpha = 0` recovers OLS.
#[cfg(feature = "linalg")]
pub fn ridge(x: &Array2<f64>, y: &Array1<f64>, alpha: f64) -> LinearModel {
    let n_samples = x.nrows();
    let n_features = x.ncols();
    assert_eq!(y.len(), n_samples, "X and y must have the same number of rows");
    assert!(alpha >= 0.0, "alpha must be non-negative");

    // Center so intercept is unpenalized and solved in closed form after β
    let x_mean = x.mean_axis(Axis(0)).expect("X must not be empty");
    let y_mean = y.mean().expect("y must not be empty");
    let x_c = x - &x_mean;
    let y_c = y - y_mean;

    // (XᵀX + α I) β = Xᵀ y
    let mut xtx = x_c.t().dot(&x_c);
    for i in 0..n_features {
        xtx[[i, i]] += alpha;
    }
    let xty = x_c.t().dot(&y_c);
    let beta = xtx
        .inv()
        .expect("Matrix inversion failed — XᵀX + αI is singular")
        .dot(&xty);

    let intercept = y_mean - x_mean.dot(&beta);
    LinearModel {
        intercept,
        coefficients: beta,
    }
}

/// Ridge regression stub without `linalg`.
#[cfg(not(feature = "linalg"))]
pub fn ridge(_x: &Array2<f64>, _y: &Array1<f64>, _alpha: f64) -> LinearModel {
    panic!(
        "symworx_stats::ridge requires the `linalg` feature. \
         Enable it in Cargo.toml with features = [\"linalg\"] on the symworx-stats dependency."
    )
}

/// Lasso regression (L1 regularized) using coordinate descent.
///
/// Returns packed coefficients `[intercept, β…]` for backward compatibility.
/// Prefer [`lasso`] for a structured [`LinearModel`].
pub fn l1(
    x: &Array2<f64>,
    y: &Array1<f64>,
    alpha: f64, // regularization strength
    max_iter: usize,
    tol: f64,
) -> Array1<f64> {
    lasso(x, y, alpha, max_iter, tol).to_packed()
}

/// Lasso regression (L1-penalized) via coordinate descent.
///
/// Minimizes `(1/2n) ‖y − Xβ − b‖² + α ‖β‖₁` with unpenalized intercept
/// (coordinate-descent soft-thresholding on centered data).
pub fn lasso(x: &Array2<f64>, y: &Array1<f64>, alpha: f64, max_iter: usize, tol: f64) -> LinearModel {
    elastic_net(x, y, alpha, 1.0, max_iter, tol)
}

/// Elastic Net regression via coordinate descent.
///
/// Minimizes
/// `(1/2n) ‖y − Xβ − b‖² + α · l1_ratio · ‖β‖₁ + ½ α · (1 − l1_ratio) · ‖β‖²`
/// (scikit-learn-style parameterization; intercept unpenalized).
///
/// # Arguments
/// * `alpha` — overall regularization strength (≥ 0)
/// * `l1_ratio` — mix of L1 vs L2 in `[0, 1]`:
///   - `1.0` → pure Lasso
///   - `0.0` → pure Ridge (coordinate-descent form)
///   - values in between → Elastic Net
pub fn elastic_net(
    x: &Array2<f64>,
    y: &Array1<f64>,
    alpha: f64,
    l1_ratio: f64,
    max_iter: usize,
    tol: f64,
) -> LinearModel {
    let n_samples = x.nrows();
    let n_features = x.ncols();
    assert_eq!(y.len(), n_samples, "X and y must have the same number of rows");
    assert!(alpha >= 0.0, "alpha must be non-negative");
    assert!((0.0..=1.0).contains(&l1_ratio), "l1_ratio must be in [0, 1]");

    let x_mean = x.mean_axis(Axis(0)).expect("X must not be empty");
    let y_mean = y.mean().expect("y must not be empty");

    let x_centered = x - &x_mean;
    let y_centered = y - y_mean;

    let n = n_samples as f64;
    let l1_pen = alpha * l1_ratio;
    let l2_pen = alpha * (1.0 - l1_ratio);

    let mut beta = Array1::<f64>::zeros(n_features);
    let mut beta_old = beta.clone();

    for _iter in 0..max_iter {
        beta_old.assign(&beta);

        for j in 0..n_features {
            // Partial residual excluding feature j
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
            // Soft-threshold with L2 denominator adjustment (Elastic Net)
            // βⱼ ← S(ρ, n·α·l1_ratio) / (‖xⱼ‖² + n·α·(1−l1_ratio))
            let denom = xj_norm + n * l2_pen;
            beta[j] = soft_threshold(rho, l1_pen * n) / denom;
        }

        let diff = (&beta - &beta_old).mapv(|v| v.abs()).sum();
        if diff < tol {
            break;
        }
    }

    let intercept = y_mean - x_mean.dot(&beta);
    LinearModel {
        intercept,
        coefficients: beta,
    }
}

/// Soft thresholding operator: S(x, λ) = sign(x) * max(|x| - λ, 0)
pub fn soft_threshold(x: f64, lambda: f64) -> f64 {
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
    use ndarray::array;

    use super::*;

    #[cfg(feature = "linalg")]
    #[test]
    fn test_l2_regression() {
        let x = array![[1.0], [2.0], [3.0], [4.0]];
        let y = array![2.0, 3.0, 5.0, 7.0];

        let coeffs = l2(&x, &y);
        assert_eq!(coeffs.len(), 2); // intercept + slope
        assert!((coeffs[0] - 0.0).abs() < 1e-9); // intercept ≈ 0
        assert!((coeffs[1] - 1.7).abs() < 1e-9); // exact slope for this data
    }

    #[cfg(feature = "linalg")]
    #[test]
    fn test_ols_predict_roundtrip() {
        let x = array![[1.0], [2.0], [3.0], [4.0]];
        let y = array![2.0, 4.0, 6.0, 8.0];
        let model = ols(&x, &y);
        let pred = model.predict(&x);
        let max_err = (&pred - &y).mapv(f64::abs).iter().cloned().fold(0.0, f64::max);
        assert!(max_err < 1e-9);
        assert!((model.intercept).abs() < 1e-9);
        assert!((model.coefficients[0] - 2.0).abs() < 1e-9);
    }

    #[cfg(feature = "linalg")]
    #[test]
    fn test_ridge_shrinks_toward_zero() {
        // Over-parameterized-ish: ridge with large α should shrink slope vs OLS
        let x = array![[1.0], [2.0], [3.0], [4.0], [5.0]];
        let y = array![1.0, 2.0, 3.0, 4.0, 5.0];

        let ols_m = ols(&x, &y);
        let ridge_m = ridge(&x, &y, 10.0);

        assert!(ridge_m.coefficients[0].abs() < ols_m.coefficients[0].abs());
        // Perfect line y=x → OLS slope ≈ 1
        assert!((ols_m.coefficients[0] - 1.0).abs() < 1e-9);
    }

    #[cfg(feature = "linalg")]
    #[test]
    fn test_ridge_alpha_zero_matches_ols() {
        // Full-rank design (second column not a scalar multiple of the first)
        let x = array![[1.0, 0.0], [2.0, 1.0], [3.0, 1.0], [4.0, 0.0], [5.0, 2.0]];
        let y = array![1.0, 3.0, 4.0, 4.0, 7.0];

        let o = ols(&x, &y);
        let r = ridge(&x, &y, 0.0);
        assert!((o.intercept - r.intercept).abs() < 1e-8);
        for i in 0..o.coefficients.len() {
            assert!((o.coefficients[i] - r.coefficients[i]).abs() < 1e-8);
        }
    }

    #[test]
    fn test_l1_basic() {
        let x = array![[1.0], [2.0], [3.0], [4.0], [5.0]];
        let y = array![2.2, 4.1, 5.9, 8.0, 10.1];

        let coeffs = l1(&x, &y, 0.1, 200, 1e-6);

        assert_eq!(coeffs.len(), 2); // intercept + 1 slope
        assert!(coeffs[1] > 1.0); // slope should be positive and reasonable
    }

    #[test]
    fn test_lasso_structured() {
        let x = array![[1.0], [2.0], [3.0], [4.0], [5.0]];
        let y = array![2.0, 4.0, 6.0, 8.0, 10.0];
        let model = lasso(&x, &y, 0.01, 500, 1e-8);
        assert_eq!(model.n_features(), 1);
        assert!(model.coefficients[0] > 1.5);
    }

    #[test]
    fn test_elastic_net_l1_ratio_one_matches_lasso() {
        let x = array![[1.0, 0.0], [2.0, 0.1], [3.0, -0.1], [4.0, 0.05], [5.0, 0.0]];
        let y = array![2.0, 4.1, 5.9, 8.05, 10.0];

        let a = lasso(&x, &y, 0.05, 500, 1e-10);
        let b = elastic_net(&x, &y, 0.05, 1.0, 500, 1e-10);

        assert!((a.intercept - b.intercept).abs() < 1e-8);
        for i in 0..a.coefficients.len() {
            assert!((a.coefficients[i] - b.coefficients[i]).abs() < 1e-8);
        }
    }

    #[test]
    fn test_soft_threshold() {
        assert!((soft_threshold(1.5, 0.5) - 1.0).abs() < 1e-15);
        assert!((soft_threshold(-1.5, 0.5) + 1.0).abs() < 1e-15);
        assert!((soft_threshold(0.3, 0.5)).abs() < 1e-15);
    }

    #[test]
    fn test_linear_model_pack_roundtrip() {
        let m = LinearModel {
            intercept: 1.5,
            coefficients: array![2.0, -0.5],
        };
        let packed = m.to_packed();
        let m2 = LinearModel::from_packed(&packed);
        assert!((m2.intercept - 1.5).abs() < 1e-15);
        assert!((m2.coefficients[0] - 2.0).abs() < 1e-15);
        assert!((m2.coefficients[1] + 0.5).abs() < 1e-15);
    }
}
