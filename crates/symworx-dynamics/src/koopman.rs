// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Extended Dynamic Mode Decomposition (EDMD) / finite-dimensional Koopman.
//!
//! Lifts states through a dictionary of observables `ψ(x)` and fits a linear
//! operator `K` on the lifted space such that
//!
//! ```text
//! ψ(x_{k+1}) ≈ K ψ(x_k)
//! ```
//!
//! (Williams, Kevrekidis & Rowley; Brunton & Kutz). When the dictionary is the
//! identity, EDMD reduces to a full-state linear model (closely related to DMD
//! without rank truncation).

use ndarray::{
    Array1,
    Array2,
    s,
};
use ndarray_linalg::{
    Eig,
    Inverse,
};
use num_complex::Complex64;

/// Observable dictionary for state lifting.
#[derive(Debug, Clone, PartialEq)]
pub enum Dictionary {
    /// `ψ(x) = x` (no lift).
    Identity,
    /// Constant + all monomials up to `max_degree` (inclusive).
    ///
    /// Degree ≥ 1 always includes the linear (state) terms, so the original
    /// state can be read from a prefix of the lifted vector after the optional
    /// constant.
    Polynomial {
        /// Maximum total degree of monomials (≥ 1 recommended).
        max_degree: usize,
        /// Prepend the constant observable `1`.
        include_constant: bool,
    },
}

impl Default for Dictionary {
    fn default() -> Self {
        Self::Polynomial {
            max_degree: 2,
            include_constant: true,
        }
    }
}

impl Dictionary {
    /// Dimension of the lifted observable vector for a state of size `n`.
    pub fn lifted_dim(&self, n: usize) -> usize {
        match self {
            Dictionary::Identity => n,
            Dictionary::Polynomial {
                max_degree,
                include_constant,
            } => {
                let mut d = if *include_constant { 1 } else { 0 };
                for deg in 1..=*max_degree {
                    d += monomial_count(n, deg);
                }
                d
            }
        }
    }
}

/// Fitted extended DMD / discrete Koopman model.
#[derive(Debug, Clone)]
pub struct EdmdResult {
    /// Koopman matrix `K` on observables (`n_obs × n_obs`): `ψ' ≈ K ψ`.
    pub k: Array2<f64>,
    /// Eigenvalues of `K`.
    pub eigenvalues: Array1<Complex64>,
    /// Right eigenvectors of `K` (columns; complex).
    pub eigenvectors: Array2<Complex64>,
    /// Dictionary used for lifting.
    pub dictionary: Dictionary,
    /// Original state dimension.
    pub state_dim: usize,
    /// Lifted observable dimension.
    pub obs_dim: usize,
    /// Training residual: mean `‖ψ' − K ψ‖₂ / max(‖ψ'‖₂, ε)` over columns.
    pub relative_fit_error: f64,
}

impl EdmdResult {
    /// Lift a state with this model's dictionary.
    pub fn lift(&self, x: &Array1<f64>) -> Array1<f64> {
        assert_eq!(x.len(), self.state_dim);
        lift_state(x, &self.dictionary)
    }

    /// One-step prediction in observable space: `ψ̂' = K ψ`.
    pub fn predict_obs(&self, psi: &Array1<f64>) -> Array1<f64> {
        assert_eq!(psi.len(), self.obs_dim);
        self.k.dot(psi)
    }

    /// Multi-step prediction of the original state (best-effort decode).
    ///
    /// Starts from `x0`, lifts, iterates `K` for `steps` steps, and decodes
    /// each lifted vector back to state coordinates via [`decode_state`].
    pub fn predict_states(&self, x0: &Array1<f64>, steps: usize) -> Vec<Array1<f64>> {
        let mut out = Vec::with_capacity(steps + 1);
        let mut psi = self.lift(x0);
        out.push(x0.to_owned());
        for _ in 0..steps {
            psi = self.predict_obs(&psi);
            out.push(decode_state(&psi, &self.dictionary, self.state_dim));
        }
        out
    }

    /// One-step state prediction: lift → `K` → decode.
    pub fn predict_one(&self, x: &Array1<f64>) -> Array1<f64> {
        let psi = self.lift(x);
        let psi_next = self.predict_obs(&psi);
        decode_state(&psi_next, &self.dictionary, self.state_dim)
    }
}

/// Configuration for [`edmd`].
#[derive(Debug, Clone)]
pub struct EdmdConfig {
    /// Observable dictionary.
    pub dictionary: Dictionary,
    /// Ridge regularization on the Gram matrix (`Y Yᵀ + λ I`). `0.0` = plain LS.
    pub ridge: f64,
}

impl Default for EdmdConfig {
    fn default() -> Self {
        Self {
            dictionary: Dictionary::default(),
            ridge: 0.0,
        }
    }
}

/// Extended DMD on a snapshot matrix (`n_state × n_time`, consecutive columns).
pub fn edmd(snapshots: &Array2<f64>, config: &EdmdConfig) -> EdmdResult {
    let m = snapshots.ncols();
    assert!(m >= 2, "need at least 2 snapshots for EDMD");
    let x = snapshots.slice(s![.., 0..m - 1]).to_owned();
    let x_prime = snapshots.slice(s![.., 1..m]).to_owned();
    edmd_pair(&x, &x_prime, config)
}

/// EDMD from paired snapshot matrices `X`, `X′` (same shape).
pub fn edmd_pair(x: &Array2<f64>, x_prime: &Array2<f64>, config: &EdmdConfig) -> EdmdResult {
    assert_eq!(x.nrows(), x_prime.nrows());
    assert_eq!(x.ncols(), x_prime.ncols());
    assert!(x.ncols() >= 1);

    let state_dim = x.nrows();
    let y = lift_snapshots(x, &config.dictionary);
    let y_prime = lift_snapshots(x_prime, &config.dictionary);
    let obs_dim = y.nrows();

    // Minimize ‖Y' − K Y‖_F  ⇒  K = Y' Yᵀ (Y Yᵀ + λ I)^{-1}
    let mut gram = y.dot(&y.t());
    if config.ridge > 0.0 {
        for i in 0..obs_dim {
            gram[[i, i]] += config.ridge;
        }
    }
    let y_prime_yt = y_prime.dot(&y.t());
    let k = y_prime_yt.dot(
        &gram
            .inv()
            .expect("EDMD Gram matrix inversion failed — try ridge > 0"),
    );

    let (evals, evecs) = k
        .eig()
        .expect("eigendecomposition of Koopman matrix failed");

    let relative_fit_error = mean_relative_column_error(&y_prime, &k.dot(&y));

    EdmdResult {
        k,
        eigenvalues: evals,
        eigenvectors: evecs,
        dictionary: config.dictionary.clone(),
        state_dim,
        obs_dim,
        relative_fit_error,
    }
}

/// Lift a single state vector.
pub fn lift_state(x: &Array1<f64>, dictionary: &Dictionary) -> Array1<f64> {
    match dictionary {
        Dictionary::Identity => x.to_owned(),
        Dictionary::Polynomial {
            max_degree,
            include_constant,
        } => {
            let n = x.len();
            let dim = dictionary.lifted_dim(n);
            let mut out = Array1::zeros(dim);
            let mut idx = 0usize;
            if *include_constant {
                out[0] = 1.0;
                idx = 1;
            }
            for deg in 1..=*max_degree {
                idx = write_monomials(x, deg, &mut out, idx);
            }
            debug_assert_eq!(idx, dim);
            out
        }
    }
}

/// Lift each column of a snapshot matrix → observables × time.
pub fn lift_snapshots(snapshots: &Array2<f64>, dictionary: &Dictionary) -> Array2<f64> {
    let n = snapshots.nrows();
    let m = snapshots.ncols();
    let d = dictionary.lifted_dim(n);
    let mut out = Array2::zeros((d, m));
    for j in 0..m {
        let col = snapshots.column(j).to_owned();
        let lifted = lift_state(&col, dictionary);
        out.column_mut(j).assign(&lifted);
    }
    out
}

/// Decode a lifted vector back to approximate state coordinates.
///
/// - [`Dictionary::Identity`]: returns `psi` unchanged.
/// - Polynomial with linear terms: reads the degree-1 block (after optional constant).
pub fn decode_state(psi: &Array1<f64>, dictionary: &Dictionary, state_dim: usize) -> Array1<f64> {
    match dictionary {
        Dictionary::Identity => {
            assert_eq!(psi.len(), state_dim);
            psi.to_owned()
        }
        Dictionary::Polynomial {
            include_constant, ..
        } => {
            let start = if *include_constant { 1 } else { 0 };
            assert!(
                psi.len() >= start + state_dim,
                "lifted vector too short to decode linear state block"
            );
            psi.slice(s![start..start + state_dim]).to_owned()
        }
    }
}

/// Number of monomials of exact total degree `deg` in `n` variables.
fn monomial_count(n: usize, deg: usize) -> usize {
    if deg == 0 {
        return 1;
    }
    if n == 0 {
        return 0;
    }
    binom(n + deg - 1, deg)
}

fn binom(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut result = 1usize;
    for i in 0..k {
        result = result * (n - i) / (i + 1);
    }
    result
}

/// Write all monomials of exact degree `deg` into `out` starting at `idx`.
/// Returns the next free index.
fn write_monomials(x: &Array1<f64>, deg: usize, out: &mut Array1<f64>, idx: usize) -> usize {
    let n = x.len();
    if n == 0 {
        return idx;
    }
    let mut next = idx;
    let mut exp = vec![0usize; n];
    fill_monomials(x, &mut exp, 0, deg, out, &mut next);
    next
}

fn fill_monomials(
    x: &Array1<f64>,
    exp: &mut [usize],
    pos: usize,
    remaining: usize,
    out: &mut Array1<f64>,
    next: &mut usize,
) {
    if pos == exp.len() - 1 {
        exp[pos] = remaining;
        let mut val = 1.0;
        for i in 0..exp.len() {
            if exp[i] > 0 {
                val *= x[i].powi(exp[i] as i32);
            }
        }
        out[*next] = val;
        *next += 1;
        return;
    }
    // Higher exponent on earlier variables first so degree-1 block is x₀, x₁, …
    for e in (0..=remaining).rev() {
        exp[pos] = e;
        fill_monomials(x, exp, pos + 1, remaining - e, out, next);
    }
}

fn mean_relative_column_error(target: &Array2<f64>, pred: &Array2<f64>) -> f64 {
    let m = target.ncols();
    if m == 0 {
        return 0.0;
    }
    let mut acc = 0.0;
    for j in 0..m {
        let t = target.column(j);
        let p = pred.column(j);
        let denom = t.dot(&t).sqrt().max(1e-15);
        let diff = &t.to_owned() - &p.to_owned();
        acc += diff.dot(&diff).sqrt() / denom;
    }
    acc / m as f64
}

// ——— Tests ———

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    fn simulate_linear(a: &Array2<f64>, x0: Array1<f64>, steps: usize) -> Array2<f64> {
        let n = x0.len();
        let mut snaps = Array2::zeros((n, steps));
        let mut x = x0;
        for k in 0..steps {
            snaps.column_mut(k).assign(&x);
            x = a.dot(&x);
        }
        snaps
    }

    #[test]
    fn test_lift_identity() {
        let x = array![1.0, 2.0];
        let y = lift_state(&x, &Dictionary::Identity);
        assert_eq!(y, x);
    }

    #[test]
    fn test_lift_polynomial_dim() {
        let dict = Dictionary::Polynomial {
            max_degree: 2,
            include_constant: true,
        };
        // n=2, deg1: 2, deg2: 3 (x², xy, y²), + const = 6
        assert_eq!(dict.lifted_dim(2), 6);
        let x = array![2.0, 3.0];
        let y = lift_state(&x, &dict);
        assert_eq!(y.len(), 6);
        assert!((y[0] - 1.0).abs() < 1e-15);
        assert!((y[1] - 2.0).abs() < 1e-15);
        assert!((y[2] - 3.0).abs() < 1e-15);
    }

    #[test]
    fn test_edmd_identity_recovers_linear_map() {
        let a = array![[0.9, 0.1], [0.0, 0.8]];
        let snaps = simulate_linear(&a, array![1.0, 0.5], 25);
        let model = edmd(
            &snaps,
            &EdmdConfig {
                dictionary: Dictionary::Identity,
                ridge: 0.0,
            },
        );
        assert_eq!(model.obs_dim, 2);
        let err = (&model.k - &a).mapv(|v| v.abs()).sum();
        assert!(err < 1e-6, "‖K − A‖₁ = {err}, K = {:?}", model.k);

        let x = array![1.0, 0.5];
        let pred = model.predict_one(&x);
        let true_next = a.dot(&x);
        let p_err = (&pred - &true_next).mapv(|v| v.abs()).sum();
        assert!(p_err < 1e-6);
    }

    #[test]
    fn test_edmd_polynomial_linear_system() {
        let a = array![[0.7]];
        let snaps = simulate_linear(&a, array![1.2], 30);
        let model = edmd(
            &snaps,
            &EdmdConfig {
                dictionary: Dictionary::Polynomial {
                    max_degree: 2,
                    include_constant: true,
                },
                ridge: 1e-10,
            },
        );
        assert!(
            model.relative_fit_error < 1e-4,
            "fit err {}",
            model.relative_fit_error
        );

        let traj = model.predict_states(&array![1.2], 5);
        let mut x = 1.2;
        for k in 0..=5 {
            assert!(
                (traj[k][0] - x).abs() < 1e-3,
                "step {k}: {} vs {x}",
                traj[k][0]
            );
            x *= 0.7;
        }
    }

    #[test]
    fn test_decode_polynomial() {
        let dict = Dictionary::Polynomial {
            max_degree: 2,
            include_constant: true,
        };
        let x = array![1.5, -0.5];
        let psi = lift_state(&x, &dict);
        let x_hat = decode_state(&psi, &dict, 2);
        assert!((x_hat[0] - 1.5).abs() < 1e-15);
        assert!((x_hat[1] + 0.5).abs() < 1e-15);
    }

    #[test]
    fn test_monomial_count() {
        assert_eq!(monomial_count(2, 1), 2);
        assert_eq!(monomial_count(2, 2), 3);
        assert_eq!(monomial_count(3, 2), 6);
    }
}
