// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Sparse Identification of Nonlinear Dynamics (SINDy).
//!
//! Discovers governing equations of the form
//!
//! ```text
//! ẋ ≈ Θ(x) Ξ
//! ```
//!
//! (or discrete maps `x⁺ ≈ Θ(x) Ξ`) by sparse regression of time derivatives
//! (or increments) against a library of candidate functions.
//!
//! The library reuses [`crate::koopman::Dictionary`] / [`crate::lift_state`].
//! Sparse coefficients are obtained by **sequential thresholded least squares
//! (STLS)**.

use ndarray::{
    Array1,
    Array2,
    s,
};
use ndarray_linalg::{
    Inverse,
    LeastSquaresSvd,
};

use crate::koopman::{
    Dictionary,
    lift_state,
};

/// Fitted SINDy model: `ẋ = Θ(x) Ξ` (columns of `xi` are per-state equations).
#[derive(Debug, Clone)]
pub struct SindyResult {
    /// Sparse coefficient matrix `Ξ` with shape `(n_library × n_state)`.
    /// Column `j` multiplies the library to produce component `j` of `ẋ`.
    pub xi: Array2<f64>,
    /// Dictionary used to build `Θ(x)`.
    pub dictionary: Dictionary,
    /// State dimension.
    pub state_dim: usize,
    /// Library (feature) dimension.
    pub library_dim: usize,
    /// Relative residual `mean_j ‖Θ ξ_j − ẋ_j‖ / ‖ẋ_j‖` on training data.
    pub relative_fit_error: f64,
    /// Number of STLS iterations performed.
    pub iterations: usize,
    /// Sample time used for finite differences (`None` if derivatives were supplied).
    pub dt: Option<f64>,
}

impl SindyResult {
    /// Evaluate the right-hand side `f(x) = Θ(x) Ξ` at a state.
    pub fn rhs(&self, x: &Array1<f64>) -> Array1<f64> {
        assert_eq!(x.len(), self.state_dim);
        let theta = lift_state(x, &self.dictionary); // length library_dim
        // f_j = θ · ξ_{·j}
        self.xi.t().dot(&theta)
    }

    /// One Euler step: `x + dt * f(x)`.
    pub fn step_euler(&self, x: &Array1<f64>, dt: f64) -> Array1<f64> {
        x + &(&self.rhs(x) * dt)
    }

    /// Simulate with forward Euler for `n_steps` from `x0`.
    pub fn simulate_euler(&self, x0: &Array1<f64>, dt: f64, n_steps: usize) -> Array2<f64> {
        let n = self.state_dim;
        let mut out = Array2::zeros((n, n_steps + 1));
        let mut x = x0.to_owned();
        out.column_mut(0).assign(&x);
        for k in 0..n_steps {
            x = self.step_euler(&x, dt);
            out.column_mut(k + 1).assign(&x);
        }
        out
    }

    /// Count non-zeros in `Ξ` above `tol`.
    pub fn sparsity(&self, tol: f64) -> usize {
        self.xi.iter().filter(|&&v| v.abs() > tol).count()
    }
}

/// Configuration for [`sindy`] / [`sindy_with_derivatives`].
#[derive(Debug, Clone)]
pub struct SindyConfig {
    /// Candidate function library.
    pub dictionary: Dictionary,
    /// STLS coefficient threshold (coefficients with `|ξ| < threshold` → 0).
    pub threshold: f64,
    /// Maximum STLS refinement iterations.
    pub max_iter: usize,
    /// Optional ridge on the normal equations (`ΘᵀΘ + λI`) for stability.
    pub ridge: f64,
}

impl Default for SindyConfig {
    fn default() -> Self {
        Self {
            dictionary: Dictionary::Polynomial {
                max_degree: 2,
                include_constant: true,
            },
            threshold: 0.1,
            max_iter: 10,
            ridge: 0.0,
        }
    }
}

/// Identify dynamics from snapshots using finite-difference derivatives.
///
/// # Arguments
/// * `snapshots` — state × time (`n × m`), consecutive columns
/// * `dt` — sample interval for `ẋ_k ≈ (x_{k+1} − x_k) / dt`
/// * `config` — library + STLS options
///
/// Uses columns `0..m-1` for `Θ(X)` and forward differences for `Ẋ`.
pub fn sindy(snapshots: &Array2<f64>, dt: f64, config: &SindyConfig) -> SindyResult {
    assert!(dt > 0.0, "dt must be positive");
    let m = snapshots.ncols();
    assert!(m >= 2, "need at least 2 snapshots");

    let x = snapshots.slice(s![.., 0..m - 1]).to_owned();
    let x_next = snapshots.slice(s![.., 1..m]).to_owned();
    let x_dot = (&x_next - &x) / dt;

    let mut result = sindy_with_derivatives(&x, &x_dot, config);
    result.dt = Some(dt);
    result
}

/// Identify dynamics when derivatives (or increments) are already available.
///
/// * `states` — `n × m` state samples (columns)
/// * `derivatives` — `n × m` corresponding `ẋ` (or `Δx`) columns
pub fn sindy_with_derivatives(states: &Array2<f64>, derivatives: &Array2<f64>, config: &SindyConfig) -> SindyResult {
    assert_eq!(states.nrows(), derivatives.nrows());
    assert_eq!(states.ncols(), derivatives.ncols());
    assert!(states.ncols() >= 1);

    let state_dim = states.nrows();

    // Library Θ: samples as rows (m × n_lib) for regression
    let theta = library_matrix_rows(states, &config.dictionary);
    let library_dim = theta.ncols();

    // X_dot as samples × state (m × n)
    let x_dot = derivatives.t().to_owned();

    let (xi, iterations) = stls(&theta, &x_dot, config.threshold, config.max_iter, config.ridge);

    let relative_fit_error = mean_relative_column_error(&x_dot, &theta.dot(&xi));

    SindyResult {
        xi,
        dictionary: config.dictionary.clone(),
        state_dim,
        library_dim,
        relative_fit_error,
        iterations,
        dt: None,
    }
}

/// Build library matrix with **rows = samples**, **cols = features**
/// from a snapshot matrix with **columns = samples**.
pub fn library_matrix_rows(snapshots: &Array2<f64>, dictionary: &Dictionary) -> Array2<f64> {
    let n = snapshots.nrows();
    let m = snapshots.ncols();
    let d = dictionary.lifted_dim(n);
    let mut theta = Array2::zeros((m, d));
    for j in 0..m {
        let x = snapshots.column(j).to_owned();
        let row = lift_state(&x, dictionary);
        theta.row_mut(j).assign(&row);
    }
    theta
}

/// Sequential thresholded least squares (classical SINDy).
///
/// `theta`: m × p, `x_dot`: m × n → `xi`: p × n
pub(crate) fn stls(
    theta: &Array2<f64>,
    x_dot: &Array2<f64>,
    threshold: f64,
    max_iter: usize,
    ridge: f64,
) -> (Array2<f64>, usize) {
    let p = theta.ncols();
    let n = x_dot.ncols();
    let mut xi = Array2::<f64>::zeros((p, n));
    let mut iterations = 0;

    for j in 0..n {
        let y = x_dot.column(j).to_owned();
        let (coef, iters) = stls_column(theta, &y, threshold, max_iter, ridge);
        iterations = iterations.max(iters);
        xi.column_mut(j).assign(&coef);
    }

    (xi, iterations)
}

fn stls_column(
    theta: &Array2<f64>,
    y: &Array1<f64>,
    threshold: f64,
    max_iter: usize,
    ridge: f64,
) -> (Array1<f64>, usize) {
    let p = theta.ncols();
    let mut active: Vec<usize> = (0..p).collect();
    let mut coef = Array1::zeros(p);
    let mut iterations = 0;

    for iter in 0..max_iter {
        iterations = iter + 1;
        if active.is_empty() {
            break;
        }

        let theta_a = select_columns(theta, &active);
        let beta_a = ridge_least_squares(&theta_a, y, ridge);

        // Write into full coefficient vector
        coef.fill(0.0);
        for (k, &idx) in active.iter().enumerate() {
            coef[idx] = beta_a[k];
        }

        // Threshold
        let mut new_active = Vec::new();
        for &idx in &active {
            if coef[idx].abs() >= threshold {
                new_active.push(idx);
            } else {
                coef[idx] = 0.0;
            }
        }

        if new_active.len() == active.len() {
            // Support unchanged — final LS on this support
            if !new_active.is_empty() {
                let theta_a = select_columns(theta, &new_active);
                let beta_a = ridge_least_squares(&theta_a, y, ridge);
                coef.fill(0.0);
                for (k, &idx) in new_active.iter().enumerate() {
                    coef[idx] = beta_a[k];
                }
            }
            break;
        }
        active = new_active;
    }

    (coef, iterations)
}

fn ridge_least_squares(theta: &Array2<f64>, y: &Array1<f64>, ridge: f64) -> Array1<f64> {
    let p = theta.ncols();
    if p == 0 {
        return Array1::zeros(0);
    }
    if ridge > 0.0 {
        // (ΘᵀΘ + λI)^{-1} Θᵀ y
        let mut gram = theta.t().dot(theta);
        for i in 0..p {
            gram[[i, i]] += ridge;
        }
        let thy = theta.t().dot(y);
        return gram.inv().expect("SINDy ridge Gram inversion failed").dot(&thy);
    }
    // Unregularized least squares via SVD
    match theta.least_squares(y) {
        Ok(sol) => sol.solution,
        Err(_) => Array1::zeros(p),
    }
}

fn select_columns(theta: &Array2<f64>, indices: &[usize]) -> Array2<f64> {
    let m = theta.nrows();
    let mut out = Array2::zeros((m, indices.len()));
    for (k, &j) in indices.iter().enumerate() {
        out.column_mut(k).assign(&theta.column(j));
    }
    out
}

pub(crate) fn mean_relative_column_error(target: &Array2<f64>, pred: &Array2<f64>) -> f64 {
    // target/pred: m × n (samples × state)
    let n = target.ncols();
    if n == 0 {
        return 0.0;
    }
    let mut acc = 0.0;
    for j in 0..n {
        let t = target.column(j);
        let p = pred.column(j);
        let denom = t.dot(&t).sqrt().max(1e-15);
        let diff = &t.to_owned() - &p.to_owned();
        acc += diff.dot(&diff).sqrt() / denom;
    }
    acc / n as f64
}

// ——— Tests ———

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;
    use crate::koopman::Dictionary;

    /// Linear system ẋ = A x with known A.
    fn simulate_linear_continuous(a: &Array2<f64>, x0: Array1<f64>, dt: f64, steps: usize) -> Array2<f64> {
        // Euler integration of ẋ = A x
        let n = x0.len();
        let mut snaps = Array2::zeros((n, steps));
        let mut x = x0;
        for k in 0..steps {
            snaps.column_mut(k).assign(&x);
            x = &x + &(a.dot(&x) * dt);
        }
        snaps
    }

    #[test]
    fn test_sindy_recovers_linear_system() {
        // ẋ = [[-0.5, 0.1], [0.0, -0.3]] x  — only linear terms
        let a = array![[-0.5, 0.1], [0.0, -0.3]];
        let dt = 0.01;
        let snaps = simulate_linear_continuous(&a, array![1.0, 0.5], dt, 400);

        let cfg = SindyConfig {
            dictionary: Dictionary::Polynomial {
                max_degree: 2,
                include_constant: true,
            },
            threshold: 0.05,
            max_iter: 15,
            ridge: 1e-10,
        };
        let model = sindy(&snaps, dt, &cfg);

        // Library layout: [1, x, y, x², xy, y²] for n=2, deg≤2
        // Linear block is indices 1,2 for each equation.
        // ξ column 0 should ≈ [0, -0.5, 0.1, 0, 0, 0]
        // ξ column 1 should ≈ [0, 0, -0.3, 0, 0, 0]
        assert_eq!(model.library_dim, 6);
        assert!(model.sparsity(1e-6) <= 4, "sparsity {}", model.sparsity(1e-6));

        let xi = &model.xi;
        assert!((xi[[1, 0]] + 0.5).abs() < 0.08, "A00: got {}", xi[[1, 0]]);
        assert!((xi[[2, 0]] - 0.1).abs() < 0.08, "A01: got {}", xi[[2, 0]]);
        assert!((xi[[2, 1]] + 0.3).abs() < 0.08, "A11: got {}", xi[[2, 1]]);
        // Constant and quadratic terms should be ~0
        assert!(xi[[0, 0]].abs() < 0.05);
        assert!(xi[[0, 1]].abs() < 0.05);
    }

    #[test]
    fn test_sindy_rhs_and_simulate() {
        let a = array![[-1.0, 0.0], [0.0, -0.5]];
        let dt = 0.02;
        let snaps = simulate_linear_continuous(&a, array![1.0, 1.0], dt, 100);
        let model = sindy(
            &snaps,
            dt,
            &SindyConfig {
                dictionary: Dictionary::Identity,
                threshold: 0.05,
                max_iter: 10,
                ridge: 1e-12,
            },
        );
        let x = array![1.0, 1.0];
        let f = model.rhs(&x);
        // ẋ ≈ A x
        assert!((f[0] + 1.0).abs() < 0.15);
        assert!((f[1] + 0.5).abs() < 0.15);

        let traj = model.simulate_euler(&x, dt, 5);
        assert_eq!(traj.ncols(), 6);
    }

    #[test]
    fn test_library_matrix_shape() {
        let snaps = array![[1.0, 2.0, 3.0], [0.0, 1.0, 0.0]];
        let dict = Dictionary::Polynomial {
            max_degree: 1,
            include_constant: true,
        };
        let theta = library_matrix_rows(&snaps, &dict);
        // const + 2 linear = 3 features, 3 samples
        assert_eq!(theta.shape(), &[3, 3]);
        assert!((theta[[0, 0]] - 1.0).abs() < 1e-15);
        assert!((theta[[0, 1]] - 1.0).abs() < 1e-15);
    }

    #[test]
    fn test_sindy_with_exact_derivatives() {
        // ẋ1 = -x1, ẋ2 = -2 x2 — exact derivatives, identity library
        let states = array![[1.0, 0.8, 0.6, 0.4], [1.0, 0.5, 0.25, 0.125]];
        let mut derivs = Array2::zeros((2, 4));
        for j in 0..4 {
            derivs[[0, j]] = -states[[0, j]];
            derivs[[1, j]] = -2.0 * states[[1, j]];
        }
        let model = sindy_with_derivatives(
            &states,
            &derivs,
            &SindyConfig {
                dictionary: Dictionary::Identity,
                threshold: 0.1,
                max_iter: 5,
                ridge: 0.0,
            },
        );
        // Ξ should be diag(-1, -2)
        assert!((model.xi[[0, 0]] + 1.0).abs() < 1e-6);
        assert!((model.xi[[1, 1]] + 2.0).abs() < 1e-6);
        assert!(model.xi[[1, 0]].abs() < 1e-6);
        assert!(model.xi[[0, 1]].abs() < 1e-6);
    }
}
