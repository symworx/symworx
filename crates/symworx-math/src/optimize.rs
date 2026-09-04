// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Lightweight optimization primitives.
//!
//! Pure-Rust gradient methods and 1-D bounded search. Intentionally free of
//! LAPACK / `ndarray-linalg` — keep this crate light.
//!
//! Typical use: nonlinear least squares in `symworx-stats`, scalar variance
//! search in mixed models ([`golden_section_minimize`]), parameter fitting
//! for dynamical models, and educational demos of gradient descent.

use ndarray::Array1;

/// Result of a gradient-descent run.
#[derive(Debug, Clone)]
pub struct GradientDescentResult {
    /// Final parameter vector.
    pub params: Array1<f64>,
    /// Objective value at the final parameters.
    pub loss: f64,
    /// Number of iterations performed.
    pub iterations: usize,
    /// `true` if the gradient-norm or step-size stopping criterion was met.
    pub converged: bool,
    /// Loss history (one entry per iteration, including the initial loss).
    pub loss_history: Vec<f64>,
}

/// Configuration for [`gradient_descent`].
#[derive(Debug, Clone)]
pub struct GradientDescentConfig {
    /// Learning rate (fixed step size when line search is disabled).
    pub learning_rate: f64,
    /// Maximum iterations.
    pub max_iter: usize,
    /// Stop when `‖∇f‖ < tol`.
    pub grad_tol: f64,
    /// Stop when parameter step `‖Δθ‖ < param_tol`.
    pub param_tol: f64,
    /// If `true`, use Armijo backtracking line search on each step.
    pub line_search: bool,
    /// Armijo sufficient-decrease constant c ∈ (0, 1).
    pub armijo_c: f64,
    /// Backtracking shrinkage factor τ ∈ (0, 1).
    pub armijo_tau: f64,
    /// Maximum backtracking steps per iteration.
    pub armijo_max_steps: usize,
}

impl Default for GradientDescentConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.01,
            max_iter: 1000,
            grad_tol: 1e-8,
            param_tol: 1e-10,
            line_search: false,
            armijo_c: 1e-4,
            armijo_tau: 0.5,
            armijo_max_steps: 20,
        }
    }
}

/// Central finite-difference gradient of a scalar objective `f: R^n → R`.
///
/// `grad[i] ≈ (f(x + ε e_i) − f(x − ε e_i)) / (2ε)`
///
/// Useful for teaching and when an analytic gradient is unavailable.
pub fn finite_difference_gradient<F>(f: &F, x: &Array1<f64>, eps: f64) -> Array1<f64>
where
    F: Fn(&Array1<f64>) -> f64,
{
    let n = x.len();
    let mut grad = Array1::zeros(n);
    let mut x_pert = x.clone();

    for i in 0..n {
        let xi = x[i];
        x_pert[i] = xi + eps;
        let f_plus = f(&x_pert);
        x_pert[i] = xi - eps;
        let f_minus = f(&x_pert);
        x_pert[i] = xi;
        grad[i] = (f_plus - f_minus) / (2.0 * eps);
    }
    grad
}

/// Gradient descent on a scalar objective with optional analytic gradient.
///
/// # Arguments
/// * `f` — objective `f(θ) → R` to minimize
/// * `grad` — optional analytic gradient; if `None`, uses
///   [`finite_difference_gradient`] with `eps = 1e-6`
/// * `x0` — initial parameter vector
/// * `config` — step size, tolerances, line search
pub fn gradient_descent<F, G>(
    f: F,
    grad: Option<&G>,
    x0: Array1<f64>,
    config: &GradientDescentConfig,
) -> GradientDescentResult
where
    F: Fn(&Array1<f64>) -> f64,
    G: Fn(&Array1<f64>) -> Array1<f64>,
{
    let mut x = x0;
    let mut loss_history = Vec::with_capacity(config.max_iter + 1);
    let mut loss = f(&x);
    loss_history.push(loss);

    let mut converged = false;
    let mut iterations = 0;

    for iter in 0..config.max_iter {
        iterations = iter + 1;

        let g = match grad {
            Some(g_fn) => g_fn(&x),
            None => finite_difference_gradient(&f, &x, 1e-6),
        };

        let grad_norm = g.dot(&g).sqrt();
        if grad_norm < config.grad_tol {
            converged = true;
            break;
        }

        let step = if config.line_search {
            armijo_step(&f, &x, loss, &g, config)
        } else {
            config.learning_rate
        };

        let x_new = &x - &(&g * step);
        let step_norm = (&x_new - &x).mapv(|v| v * v).sum().sqrt();
        x = x_new;
        loss = f(&x);
        loss_history.push(loss);

        if step_norm < config.param_tol {
            converged = true;
            break;
        }
    }

    // Final gradient check (in case we hit max_iter with tiny gradient)
    if !converged {
        let g = match grad {
            Some(g_fn) => g_fn(&x),
            None => finite_difference_gradient(&f, &x, 1e-6),
        };
        if g.dot(&g).sqrt() < config.grad_tol {
            converged = true;
        }
    }

    GradientDescentResult {
        params: x,
        loss,
        iterations,
        converged,
        loss_history,
    }
}

/// Result of a 1-D bounded scalar search ([`golden_section_minimize`]).
#[derive(Debug, Clone, PartialEq)]
pub struct ScalarSearchResult {
    /// Location of the reported extremum (NaN if the interval was invalid).
    pub x: f64,
    /// Objective value `f(x)` (NaN if invalid).
    pub fx: f64,
    /// Iterations performed.
    pub iterations: usize,
    /// `true` if the interval width shrank below `tol`.
    pub converged: bool,
}

fn golden_section_invalid() -> ScalarSearchResult {
    ScalarSearchResult {
        x: f64::NAN,
        fx: f64::NAN,
        iterations: 0,
        converged: false,
    }
}

/// Golden-section search for a minimum of a unimodal `f` on `[lo, hi]`.
///
/// Invalid interval (`lo >= hi` or non-finite bounds) returns
/// `converged: false` and NaN `x` / `fx` (no panic).
pub fn golden_section_minimize<F>(f: F, lo: f64, hi: f64, tol: f64, max_iter: usize) -> ScalarSearchResult
where
    F: Fn(f64) -> f64,
{
    if !lo.is_finite() || !hi.is_finite() || !tol.is_finite() || lo >= hi || max_iter == 0 {
        return golden_section_invalid();
    }

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
        ScalarSearchResult {
            x: mid,
            fx: fm,
            iterations,
            converged,
        }
    } else {
        ScalarSearchResult {
            x,
            fx,
            iterations,
            converged,
        }
    }
}

/// Golden-section search for a maximum of a unimodal `f` on `[lo, hi]`.
///
/// Implemented as [`golden_section_minimize`] of `-f`. `fx` is `f(x)`, not `-f(x)`.
pub fn golden_section_maximize<F>(f: F, lo: f64, hi: f64, tol: f64, max_iter: usize) -> ScalarSearchResult
where
    F: Fn(f64) -> f64,
{
    let mut r = golden_section_minimize(|x| -f(x), lo, hi, tol, max_iter);
    if r.fx.is_finite() {
        r.fx = -r.fx;
    }
    r
}

/// Convenience: gradient descent with finite-difference gradients only.
pub fn gradient_descent_fd<F>(f: F, x0: Array1<f64>, config: &GradientDescentConfig) -> GradientDescentResult
where
    F: Fn(&Array1<f64>) -> f64,
{
    // Turbofish supplies a dummy G so `None` type-checks as `Option<&G>`.
    gradient_descent(f, None::<&fn(&Array1<f64>) -> Array1<f64>>, x0, config)
}

/// Armijo backtracking: find step length `α` along `−∇f` satisfying
/// `f(x − α g) ≤ f(x) − c α ‖g‖²`.
///
/// Armijo knobs come from [`GradientDescentConfig`] so this stays under
/// Clippy's argument-count limit.
fn armijo_step<F>(f: &F, x: &Array1<f64>, f_x: f64, g: &Array1<f64>, config: &GradientDescentConfig) -> f64
where
    F: Fn(&Array1<f64>) -> f64,
{
    let mut alpha = config.learning_rate;
    let g_norm_sq = g.dot(g);

    for _ in 0..config.armijo_max_steps {
        let x_trial = x - &(g * alpha);
        let f_trial = f(&x_trial);
        if f_trial <= f_x - config.armijo_c * alpha * g_norm_sq {
            return alpha;
        }
        alpha *= config.armijo_tau;
    }
    alpha
}

// ——— Tests ———

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    #[test]
    fn test_fd_gradient_quadratic() {
        // f(x) = 0.5 (x−1)² + 0.5 (y−2)²  →  ∇f = (x−1, y−2)
        let f = |p: &Array1<f64>| 0.5 * (p[0] - 1.0).powi(2) + 0.5 * (p[1] - 2.0).powi(2);
        let x = array![3.0, 0.0];
        let g = finite_difference_gradient(&f, &x, 1e-6);
        assert!((g[0] - 2.0).abs() < 1e-5);
        assert!((g[1] + 2.0).abs() < 1e-5);
    }

    #[test]
    fn test_gd_quadratic_analytic() {
        let f = |p: &Array1<f64>| (p[0] - 3.0).powi(2) + (p[1] + 1.0).powi(2);
        let g = |p: &Array1<f64>| array![2.0 * (p[0] - 3.0), 2.0 * (p[1] + 1.0)];

        let cfg = GradientDescentConfig {
            learning_rate: 0.2,
            max_iter: 200,
            grad_tol: 1e-10,
            ..Default::default()
        };

        let result = gradient_descent(f, Some(&g), array![0.0, 0.0], &cfg);
        assert!(result.converged);
        assert!((result.params[0] - 3.0).abs() < 1e-6);
        assert!((result.params[1] + 1.0).abs() < 1e-6);
        assert!(result.loss < 1e-12);
    }

    #[test]
    fn test_gd_fd_rosenbrock_ish() {
        // Simple bowl — FD should find the minimum
        let f = |p: &Array1<f64>| (p[0] - 1.0).powi(2) + 4.0 * (p[1] - 2.0).powi(2);
        let cfg = GradientDescentConfig {
            learning_rate: 0.1,
            max_iter: 500,
            grad_tol: 1e-8,
            line_search: true,
            ..Default::default()
        };
        let result = gradient_descent_fd(f, array![0.0, 0.0], &cfg);
        assert!((result.params[0] - 1.0).abs() < 1e-4);
        assert!((result.params[1] - 2.0).abs() < 1e-4);
    }

    #[test]
    fn test_armijo_reduces_loss() {
        let f = |p: &Array1<f64>| p[0].powi(2);
        let cfg = GradientDescentConfig {
            learning_rate: 10.0, // too large without line search
            max_iter: 50,
            line_search: true,
            grad_tol: 1e-10,
            ..Default::default()
        };
        let result = gradient_descent_fd(f, array![5.0], &cfg);
        assert!(result.loss < 1e-8);
        assert!(result.params[0].abs() < 1e-4);
    }

    #[test]
    fn golden_section_quadratic_min() {
        let f = |x: f64| (x - 3.0).powi(2);
        let r = golden_section_minimize(f, 0.0, 5.0, 1e-10, 80);
        assert!(r.converged);
        assert!((r.x - 3.0).abs() < 1e-8);
        assert!(r.fx.abs() < 1e-14);
    }

    #[test]
    fn golden_section_quadratic_max() {
        let f = |x: f64| -(x - 3.0).powi(2);
        let r = golden_section_maximize(f, 0.0, 5.0, 1e-10, 80);
        assert!(r.converged);
        assert!((r.x - 3.0).abs() < 1e-8);
        assert!(r.fx.abs() < 1e-14);
    }

    #[test]
    fn golden_section_invalid_interval() {
        let f = |x: f64| x * x;
        let r = golden_section_minimize(f, 2.0, 1.0, 1e-8, 20);
        assert!(!r.converged);
        assert!(r.x.is_nan());
        assert_eq!(r.iterations, 0);
    }
}
