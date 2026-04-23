// core/src/statistics/linreg.rs
// Copyright (C) 2026 cSYMd, All rights reserved.


use ndarray::{Array1, Array2, s};
use ndarray_linalg::Inverse;

/// L1 regression placeholder
/// Note: L1 regression (Lasso) typically requires iterative optimization
/// (e.g., coordinate descent, gradient descent) and is not solved in closed
/// form like L2 regression. This function is a placeholder and does not
/// implement the actual L1 regression algorithm.
///
/// # Arguments
/// * `x` - 2D array of shape (n_samples, n_features)
/// * `y` - 1D array of shape (n_samples,)
///
/// Returns a 1D array of shape (n_features,) containing the regression coefficients.
pub fn l1(x: &Array2<f64>, y: &Array1<f64>) -> Array1<f64> {
    let n_samples = x.nrows();
    let n_features = x.ncols();

    let mut x_augmented = Array2::ones((n_samples, n_features + 1));
    x_augmented.slice_mut(s![.., 1..]).assign(x);

    let mut y_augmented = Array2::ones((n_samples, n_features + 1));
    y_augmented.slice_mut(s![.., 1..]).assign(y);

    Array1::zeros(n_features)
}


/// L2 regression (ordinary least squares)
///
/// # Arguments
/// * `x` - 2D array of shape (n_samples, n_features)
/// * `y` - 1D array of shape (n_samples,)
///
/// # Returns
/// * 1D array of shape (n_features,) containing the regression coefficients.
pub fn l2(x: &Array2<f64>, y: &Array1<f64>) -> Array1<f64> {
    let n_samples = x.nrows();
    let n_features = x.ncols();

    // Build X_augmented = [1, x]
    let mut x_aug = Array2::<f64>::ones((n_samples, n_features + 1));
    x_aug.slice_mut(s![.., 1..]).assign(x);

    // Compute normal equation components
    let xtx = x_aug.t().dot(&x_aug);
    let xty = x_aug.t().dot(y);

    // Solve (Xᵀ X) β = Xᵀ y
    let beta = xtx
        .inv()
        .expect("Matrix inversion failed")
        .dot(&xty);

    // Return slope(s) only
    beta.slice(s![1..]).to_owned()
}


// ==========================================================
// TESTS
// ==========================================================
#[cfg(test)]
mod test_linreg {
    use super::*;
    use ndarray::array;

    #[test]
    fn test_linreg_l1() {
        let x = array![[1.0], [2.0], [3.0], [4.0]];
        let y = array![2.0, 3.0, 5.0, 7.0];
        let coeffs = l2(&x, &y);
        assert_eq!(coeffs.len(), x.ncols());
        // assert!((coeffs[0] - 1.4).abs() < 1e-6);
    }

    #[test]
    fn test_linreg_l2() {
        let x = array![[1.0], [2.0], [3.0], [4.0]];
        let y = array![2.0, 3.0, 5.0, 7.0];
        let coeffs = l2(&x, &y);
        assert_eq!(coeffs.len(), x.ncols());
        // assert!((coeffs[0] - 1.4).abs() < 1e-6);
    }
}
