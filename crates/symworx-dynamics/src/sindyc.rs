// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Sparse Identification of Nonlinear Dynamics with **Control** (SINDYc).
//!
//! Discovers forced dynamics
//!
//! ```text
//! ẋ ≈ Θ(x, u) Ξ
//! ```
//!
//! by sparse regression against a library that depends on both state and
//! control (Brunton, Proctor & Kutz, *Proc. Natl. Acad. Sci.* / DDSE).
//!
//! Library layout (configurable):
//!
//! 1. **State block** — `ψ_x(x)` from [`Dictionary`] (same as SINDy)
//! 2. **Control block** — `ψ_u(u)` (constant term dropped if both blocks include one)
//! 3. **Optional products** — `ψ_x^{(i)} · ψ_u^{(j)}` for non-constant features
//!
//! Sparse coefficients use the same **STLS** solver as [`crate::sindy`].

use ndarray::{
    Array1,
    Array2,
    s,
};

use crate::koopman::{
    Dictionary,
    lift_state,
};
use crate::sindy::{
    mean_relative_column_error,
    stls,
};

/// Fitted SINDYc model: `ẋ = Θ(x, u) Ξ`.
#[derive(Debug, Clone)]
pub struct SindycResult {
    /// Sparse coefficients `(n_library × n_state)`.
    pub xi: Array2<f64>,
    /// State dictionary used in the library.
    pub state_dictionary: Dictionary,
    /// Control dictionary used in the library.
    pub control_dictionary: Dictionary,
    /// Whether state–control product terms were included.
    pub include_products: bool,
    /// State dimension `n`.
    pub state_dim: usize,
    /// Control dimension `m`.
    pub control_dim: usize,
    /// Total library width.
    pub library_dim: usize,
    /// Relative residual on training data.
    pub relative_fit_error: f64,
    /// STLS iterations (max over state equations).
    pub iterations: usize,
    /// Sample time if finite differences were used.
    pub dt: Option<f64>,
}

impl SindycResult {
    /// Evaluate `f(x, u) = Θ(x, u) Ξ`.
    pub fn rhs(&self, x: &Array1<f64>, u: &Array1<f64>) -> Array1<f64> {
        assert_eq!(x.len(), self.state_dim);
        assert_eq!(u.len(), self.control_dim);
        let theta = lift_xu(
            x,
            u,
            &self.state_dictionary,
            &self.control_dictionary,
            self.include_products,
        );
        self.xi.t().dot(&theta)
    }

    /// One Euler step: `x + dt · f(x, u)`.
    pub fn step_euler(&self, x: &Array1<f64>, u: &Array1<f64>, dt: f64) -> Array1<f64> {
        x + &(&self.rhs(x, u) * dt)
    }

    /// Simulate with a prescribed control sequence.
    ///
    /// * `controls` — length `n_steps`; `controls[k]` is applied at step `k`
    /// * returns states `n × (n_steps + 1)`
    pub fn simulate_euler(
        &self,
        x0: &Array1<f64>,
        controls: &[Array1<f64>],
        dt: f64,
    ) -> Array2<f64> {
        let n_steps = controls.len();
        let mut out = Array2::zeros((self.state_dim, n_steps + 1));
        let mut x = x0.to_owned();
        out.column_mut(0).assign(&x);
        for (k, u) in controls.iter().enumerate() {
            x = self.step_euler(&x, u, dt);
            out.column_mut(k + 1).assign(&x);
        }
        out
    }

    /// Non-zero coefficient count above `tol`.
    pub fn sparsity(&self, tol: f64) -> usize {
        self.xi.iter().filter(|&&v| v.abs() > tol).count()
    }

    /// Lift `(x, u)` with this model's library (diagnostics / teaching).
    pub fn lift(&self, x: &Array1<f64>, u: &Array1<f64>) -> Array1<f64> {
        lift_xu(
            x,
            u,
            &self.state_dictionary,
            &self.control_dictionary,
            self.include_products,
        )
    }
}

/// Configuration for [`sindyc`] / [`sindyc_with_derivatives`].
#[derive(Debug, Clone)]
pub struct SindycConfig {
    /// Library for the state `x`.
    pub state_dictionary: Dictionary,
    /// Library for the control `u`.
    pub control_dictionary: Dictionary,
    /// Append products of non-constant state features with non-constant control features.
    pub include_products: bool,
    /// STLS hard-threshold.
    pub threshold: f64,
    /// Max STLS iterations.
    pub max_iter: usize,
    /// Ridge on normal equations.
    pub ridge: f64,
}

impl Default for SindycConfig {
    fn default() -> Self {
        Self {
            state_dictionary: Dictionary::Polynomial {
                max_degree: 2,
                include_constant: true,
            },
            // Affine in control by default: [1 is dropped if state has const] + u
            control_dictionary: Dictionary::Identity,
            include_products: true,
            threshold: 0.1,
            max_iter: 10,
            ridge: 0.0,
        }
    }
}

/// SINDYc from snapshots + controls using finite-difference derivatives.
///
/// # Arguments
/// * `snapshots` — state × time (`n × T`), consecutive columns
/// * `controls` — control × time (`m × T`); column `k` is `u_k` applied while
///   in state column `k` (same length as snapshots; last control unused for FD)
/// * `dt` — sample interval
/// * `config` — library + STLS options
pub fn sindyc(
    snapshots: &Array2<f64>,
    controls: &Array2<f64>,
    dt: f64,
    config: &SindycConfig,
) -> SindycResult {
    assert!(dt > 0.0, "dt must be positive");
    let t = snapshots.ncols();
    assert!(t >= 2, "need at least 2 snapshots");
    assert_eq!(
        controls.ncols(),
        t,
        "controls must have the same number of columns as snapshots"
    );

    let x = snapshots.slice(s![.., 0..t - 1]).to_owned();
    let x_next = snapshots.slice(s![.., 1..t]).to_owned();
    let u = controls.slice(s![.., 0..t - 1]).to_owned();
    let x_dot = (&x_next - &x) / dt;

    let mut result = sindyc_with_derivatives(&x, &u, &x_dot, config);
    result.dt = Some(dt);
    result
}

/// SINDYc when derivatives are already available.
///
/// * `states` — `n × N` samples  
/// * `controls` — `m × N` samples (aligned with states)  
/// * `derivatives` — `n × N` values of `ẋ`
pub fn sindyc_with_derivatives(
    states: &Array2<f64>,
    controls: &Array2<f64>,
    derivatives: &Array2<f64>,
    config: &SindycConfig,
) -> SindycResult {
    assert_eq!(states.ncols(), controls.ncols());
    assert_eq!(states.ncols(), derivatives.ncols());
    assert_eq!(states.nrows(), derivatives.nrows());
    assert!(states.ncols() >= 1);

    let state_dim = states.nrows();
    let control_dim = controls.nrows();

    let theta = library_matrix_xu(
        states,
        controls,
        &config.state_dictionary,
        &config.control_dictionary,
        config.include_products,
    );
    let library_dim = theta.ncols();
    let x_dot = derivatives.t().to_owned();

    let (xi, iterations) = stls(
        &theta,
        &x_dot,
        config.threshold,
        config.max_iter,
        config.ridge,
    );

    let relative_fit_error = mean_relative_column_error(&x_dot, &theta.dot(&xi));

    SindycResult {
        xi,
        state_dictionary: config.state_dictionary.clone(),
        control_dictionary: config.control_dictionary.clone(),
        include_products: config.include_products,
        state_dim,
        control_dim,
        library_dim,
        relative_fit_error,
        iterations,
        dt: None,
    }
}

/// Library matrix with **rows = samples**: `Θ(x_k, u_k)`.
pub fn library_matrix_xu(
    states: &Array2<f64>,
    controls: &Array2<f64>,
    state_dict: &Dictionary,
    control_dict: &Dictionary,
    include_products: bool,
) -> Array2<f64> {
    let n_samples = states.ncols();
    assert_eq!(controls.ncols(), n_samples);
    let d = library_dim_xu(
        states.nrows(),
        controls.nrows(),
        state_dict,
        control_dict,
        include_products,
    );
    let mut theta = Array2::zeros((n_samples, d));
    for k in 0..n_samples {
        let x = states.column(k).to_owned();
        let u = controls.column(k).to_owned();
        let row = lift_xu(&x, &u, state_dict, control_dict, include_products);
        theta.row_mut(k).assign(&row);
    }
    theta
}

/// Dimension of the controlled library for given state/control sizes.
pub fn library_dim_xu(
    state_dim: usize,
    control_dim: usize,
    state_dict: &Dictionary,
    control_dict: &Dictionary,
    include_products: bool,
) -> usize {
    let dx = state_dict.lifted_dim(state_dim);
    let du_full = control_dict.lifted_dim(control_dim);
    let drop_u_const = has_constant(state_dict) && has_constant(control_dict);
    let du = if drop_u_const {
        du_full.saturating_sub(1)
    } else {
        du_full
    };

    let mut d = dx + du;
    if include_products {
        let sx = nonconst_start(state_dict);
        let nx = dx.saturating_sub(sx);
        // After optional drop of control constant, product uses every remaining control feature.
        let nu = if drop_u_const {
            du
        } else {
            du_full.saturating_sub(nonconst_start(control_dict))
        };
        d += nx * nu;
    }
    d
}

/// Lift a single `(x, u)` pair into the SINDYc library vector.
pub fn lift_xu(
    x: &Array1<f64>,
    u: &Array1<f64>,
    state_dict: &Dictionary,
    control_dict: &Dictionary,
    include_products: bool,
) -> Array1<f64> {
    let psi_x = lift_state(x, state_dict);
    let psi_u_full = lift_state(u, control_dict);

    let drop_u_const = has_constant(state_dict) && has_constant(control_dict);
    let psi_u: Array1<f64> = if drop_u_const && !psi_u_full.is_empty() {
        psi_u_full.slice(s![1..]).to_owned()
    } else {
        psi_u_full.clone()
    };

    let mut parts: Vec<f64> = Vec::with_capacity(psi_x.len() + psi_u.len() + 16);
    parts.extend(psi_x.iter().copied());
    parts.extend(psi_u.iter().copied());

    if include_products {
        let sx = nonconst_start(state_dict);
        // Products: non-constant state features × each control feature in psi_u
        // (psi_u already has const dropped when both had constants)
        for i in sx..psi_x.len() {
            for j in 0..psi_u.len() {
                parts.push(psi_x[i] * psi_u[j]);
            }
        }
    }

    Array1::from_vec(parts)
}

fn has_constant(dict: &Dictionary) -> bool {
    match dict {
        Dictionary::Identity => false,
        Dictionary::Polynomial {
            include_constant, ..
        } => *include_constant,
    }
}

fn nonconst_start(dict: &Dictionary) -> usize {
    if has_constant(dict) {
        1
    } else {
        0
    }
}

// ——— Tests ———

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    /// ẋ = A x + B u  (Euler)
    fn simulate_forced(
        a: &Array2<f64>,
        b: &Array2<f64>,
        x0: Array1<f64>,
        controls: &Array2<f64>,
        dt: f64,
    ) -> Array2<f64> {
        let n = x0.len();
        let t = controls.ncols();
        let mut snaps = Array2::zeros((n, t));
        let mut x = x0;
        for k in 0..t {
            snaps.column_mut(k).assign(&x);
            let u = controls.column(k).to_owned();
            let dx = a.dot(&x) + b.dot(&u);
            x = &x + &(&dx * dt);
        }
        snaps
    }

    #[test]
    fn test_lift_xu_dims() {
        let x = array![1.0, 2.0];
        let u = array![0.5];
        let sd = Dictionary::Polynomial {
            max_degree: 1,
            include_constant: true,
        };
        let cd = Dictionary::Identity;
        // state: 1, x, y (3); control: u (1); products: x*u, y*u (2) → 6
        let row = lift_xu(&x, &u, &sd, &cd, true);
        assert_eq!(row.len(), 6);
        assert!((row[0] - 1.0).abs() < 1e-15);
        assert!((row[1] - 1.0).abs() < 1e-15);
        assert!((row[2] - 2.0).abs() < 1e-15);
        assert!((row[3] - 0.5).abs() < 1e-15); // u
        assert!((row[4] - 0.5).abs() < 1e-15); // x*u
        assert!((row[5] - 1.0).abs() < 1e-15); // y*u
    }

    #[test]
    fn test_sindyc_recovers_ab() {
        // ẋ = -0.5 x + 1.0 u   (scalar)
        let a = array![[-0.5]];
        let b = array![[1.0]];
        let dt = 0.01;
        let t = 500;
        // Persistent excitation: multi-sine control
        let mut controls = Array2::zeros((1, t));
        for k in 0..t {
            let tk = k as f64 * dt;
            controls[[0, k]] = (2.0 * std::f64::consts::PI * 0.5 * tk).sin()
                + 0.3 * (2.0 * std::f64::consts::PI * 1.3 * tk).sin();
        }
        let snaps = simulate_forced(&a, &b, array![0.2], &controls, dt);

        let cfg = SindycConfig {
            state_dictionary: Dictionary::Polynomial {
                max_degree: 2,
                include_constant: true,
            },
            control_dictionary: Dictionary::Identity,
            include_products: false, // affine: Θ = [1, x, x², u]
            threshold: 0.05,
            max_iter: 15,
            ridge: 1e-10,
        };
        let model = sindyc(&snaps, &controls, dt, &cfg);

        // Library: [1, x, x², u] → indices 0,1,2,3
        // Expect ξ ≈ [0, -0.5, 0, 1.0]
        let xi = &model.xi;
        assert_eq!(xi.ncols(), 1);
        assert!(
            (xi[[1, 0]] + 0.5).abs() < 0.1,
            "A coeff {}, xi={:?}",
            xi[[1, 0]],
            xi.column(0)
        );
        assert!(
            (xi[[3, 0]] - 1.0).abs() < 0.15,
            "B coeff {}, xi={:?}",
            xi[[3, 0]],
            xi.column(0)
        );
        assert!(xi[[0, 0]].abs() < 0.08, "const");
        assert!(xi[[2, 0]].abs() < 0.08, "x²");
    }

    #[test]
    fn test_sindyc_exact_derivatives_affine() {
        // ẋ = -x + 2 u, exact ẋ
        let states = array![[1.0, 0.5, 0.0, -0.5]];
        let controls = array![[0.0, 1.0, 0.5, -1.0]];
        let mut derivs = Array2::zeros((1, 4));
        for k in 0..4 {
            derivs[[0, k]] = -states[[0, k]] + 2.0 * controls[[0, k]];
        }
        let model = sindyc_with_derivatives(
            &states,
            &controls,
            &derivs,
            &SindycConfig {
                state_dictionary: Dictionary::Identity,
                control_dictionary: Dictionary::Identity,
                include_products: false,
                threshold: 0.1,
                max_iter: 5,
                ridge: 0.0,
            },
        );
        // Θ = [x, u], Ξ = [-1, 2]^T
        assert!((model.xi[[0, 0]] + 1.0).abs() < 1e-6);
        assert!((model.xi[[1, 0]] - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_sindyc_simulate_matches_rhs() {
        let states = array![[1.0, 0.5]];
        let controls = array![[0.5, 0.0]];
        let mut derivs = Array2::zeros((1, 2));
        for k in 0..2 {
            derivs[[0, k]] = -0.5 * states[[0, k]] + controls[[0, k]];
        }
        let model = sindyc_with_derivatives(
            &states,
            &controls,
            &derivs,
            &SindycConfig {
                state_dictionary: Dictionary::Identity,
                control_dictionary: Dictionary::Identity,
                include_products: false,
                threshold: 0.05,
                max_iter: 5,
                ridge: 0.0,
            },
        );
        let x = array![1.0];
        let u = array![0.5];
        let f = model.rhs(&x, &u);
        assert!((f[0] - (-0.5 + 0.5)).abs() < 1e-5);

        let us = vec![array![0.0], array![1.0]];
        let traj = model.simulate_euler(&x, &us, 0.1);
        assert_eq!(traj.ncols(), 3);
    }

    #[test]
    fn test_library_dim_consistency() {
        let states = array![[1.0, 2.0], [0.0, 1.0]];
        let controls = array![[0.5, -0.5]];
        let sd = Dictionary::Polynomial {
            max_degree: 1,
            include_constant: true,
        };
        let cd = Dictionary::Identity;
        let theta = library_matrix_xu(&states, &controls, &sd, &cd, true);
        let d = library_dim_xu(2, 1, &sd, &cd, true);
        assert_eq!(theta.ncols(), d);
        assert_eq!(theta.nrows(), 2);
    }
}
