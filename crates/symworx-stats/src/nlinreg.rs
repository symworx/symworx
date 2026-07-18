// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Nonlinear least-squares regression via gradient descent.
//!
//! Fits a parametric model `y ≈ f(x; θ)` by minimizing the residual sum of
//! squares. Uses optimization primitives from `symworx-math` (no LAPACK).

use ndarray::{
    Array1,
    Array2,
};
use symworx_math::optimize::{
    GradientDescentConfig,
    GradientDescentResult,
    gradient_descent_fd,
};

/// Result of a nonlinear least-squares fit.
#[derive(Debug, Clone)]
pub struct NonlinearFitResult {
    /// Estimated parameters `θ`.
    pub params: Array1<f64>,
    /// Residual sum of squares at the solution.
    pub rss: f64,
    /// Root-mean-square residual.
    pub rmse: f64,
    /// Gradient-descent diagnostics.
    pub opt: GradientDescentResult,
}

/// Fit parameters of a scalar model by nonlinear least squares.
///
/// Minimizes `Σᵢ (f(xᵢ; θ) − yᵢ)²` with gradient descent (analytic residual
/// Jacobian optional via finite differences on the RSS).
///
/// # Arguments
/// * `x` — independent variable samples (length n)
/// * `y` — observed responses (length n)
/// * `model` — `model(x_i, θ) → ŷ_i`
/// * `theta0` — initial parameter guess
/// * `config` — optimizer settings (learning rate, line search, …)
///
/// # Example
/// ```ignore
/// // Fit y ≈ a * exp(b * x)
/// let model = |xi: f64, th: &Array1<f64>| th[0] * (th[1] * xi).exp();
/// let fit = nonlinear_least_squares(&x, &y, model, array![1.0, 0.1], &cfg);
/// ```
pub fn nonlinear_least_squares<M>(
    x: &[f64],
    y: &[f64],
    model: M,
    theta0: Array1<f64>,
    config: &GradientDescentConfig,
) -> NonlinearFitResult
where
    M: Fn(f64, &Array1<f64>) -> f64,
{
    assert_eq!(x.len(), y.len(), "x and y must have the same length");
    let n = x.len() as f64;

    let loss = |theta: &Array1<f64>| {
        let mut rss = 0.0;
        for i in 0..x.len() {
            let r = model(x[i], theta) - y[i];
            rss += r * r;
        }
        rss
    };

    let opt = gradient_descent_fd(loss, theta0, config);
    let rss = opt.loss;
    let rmse = if n > 0.0 { (rss / n).sqrt() } else { 0.0 };

    NonlinearFitResult {
        params: opt.params.clone(),
        rss,
        rmse,
        opt,
    }
}

/// Multivariate design-matrix form: `model(x_row, θ) → ŷ`.
///
/// Each row of `x` is one sample's feature vector.
pub fn nonlinear_least_squares_design<M>(
    x: &Array2<f64>,
    y: &Array1<f64>,
    model: M,
    theta0: Array1<f64>,
    config: &GradientDescentConfig,
) -> NonlinearFitResult
where
    M: Fn(&Array1<f64>, &Array1<f64>) -> f64,
{
    assert_eq!(
        x.nrows(),
        y.len(),
        "X and y must have the same number of rows"
    );
    let n = y.len() as f64;

    let loss = |theta: &Array1<f64>| {
        let mut rss = 0.0;
        for i in 0..x.nrows() {
            let row = x.row(i).to_owned();
            let r = model(&row, theta) - y[i];
            rss += r * r;
        }
        rss
    };

    let opt = gradient_descent_fd(loss, theta0, config);
    let rss = opt.loss;
    let rmse = if n > 0.0 { (rss / n).sqrt() } else { 0.0 };

    NonlinearFitResult {
        params: opt.params.clone(),
        rss,
        rmse,
        opt,
    }
}

#[cfg(test)]
mod tests {
    use ndarray::array;
    use symworx_math::optimize::GradientDescentConfig;

    use super::*;

    #[test]
    fn test_fit_linear_as_nonlinear() {
        // y = 2x + 1 — recover with GD from a bad start
        let x: Vec<f64> = (0..20).map(|i| i as f64 * 0.5).collect();
        let y: Vec<f64> = x.iter().map(|&xi| 2.0 * xi + 1.0).collect();

        let model = |xi: f64, th: &Array1<f64>| th[0] * xi + th[1];
        let cfg = GradientDescentConfig {
            learning_rate: 0.01,
            max_iter: 2000,
            grad_tol: 1e-10,
            line_search: true,
            ..Default::default()
        };

        let fit = nonlinear_least_squares(&x, &y, model, array![0.0, 0.0], &cfg);
        assert!((fit.params[0] - 2.0).abs() < 1e-3, "slope {:?}", fit.params);
        assert!(
            (fit.params[1] - 1.0).abs() < 1e-3,
            "intercept {:?}",
            fit.params
        );
        assert!(fit.rmse < 1e-3);
    }

    #[test]
    fn test_fit_exponential() {
        let x: Vec<f64> = (0..15).map(|i| i as f64 * 0.2).collect();
        let a_true = 1.5;
        let b_true = 0.4;
        let y: Vec<f64> = x.iter().map(|&xi| a_true * (b_true * xi).exp()).collect();

        let model = |xi: f64, th: &Array1<f64>| th[0] * (th[1] * xi).exp();
        let cfg = GradientDescentConfig {
            learning_rate: 0.05,
            max_iter: 3000,
            line_search: true,
            grad_tol: 1e-10,
            ..Default::default()
        };

        let fit = nonlinear_least_squares(&x, &y, model, array![1.0, 0.1], &cfg);
        assert!(
            (fit.params[0] - a_true).abs() < 0.05,
            "a = {}",
            fit.params[0]
        );
        assert!(
            (fit.params[1] - b_true).abs() < 0.05,
            "b = {}",
            fit.params[1]
        );
    }
}
