// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Dynamic Mode Decomposition (DMD).
//!
//! SVD-based exact DMD (Schmid / Tu et al. / Brunton & Kutz) for discrete-time
//! linear (or locally linear) dynamics on snapshot matrices.
//!
//! Given consecutive snapshot matrices `X` and `X′` (each column is a state),
//! DMD finds eigenvalues and spatial modes of an approximate linear map
//! `x_{k+1} ≈ A x_k` without forming `A` explicitly in the full space.

use ndarray::{
    Array1,
    Array2,
    s,
};
use ndarray_linalg::{
    Eig,
    LeastSquaresSvd,
};
use num_complex::Complex64;
use symworx_stats::svd::Svd;

/// Result of a Dynamic Mode Decomposition.
#[derive(Debug, Clone)]
pub struct DmdResult {
    /// DMD eigenvalues λ (discrete-time; length r).
    pub eigenvalues: Array1<Complex64>,
    /// DMD modes Φ (n_state × r), complex.
    pub modes: Array2<Complex64>,
    /// Mode amplitudes b from fitting the first snapshot: `x₀ ≈ Φ b`.
    pub amplitudes: Array1<Complex64>,
    /// Singular values of the data matrix `X` (full, before truncation).
    pub singular_values: Array1<f64>,
    /// Rank used (number of modes retained).
    pub rank: usize,
    /// Optional continuous-time eigenvalues `ω = log(λ) / Δt` when `dt` was set.
    pub omega: Option<Array1<Complex64>>,
}

impl DmdResult {
    /// Predict the state at discrete step `k` (k = 0 is the first snapshot):
    /// `x_k ≈ Φ diag(λ^k) b` (real part returned).
    pub fn predict_discrete(&self, k: usize) -> Array1<f64> {
        let r = self.rank;
        let n = self.modes.nrows();
        let mut x = Array1::<Complex64>::zeros(n);
        for j in 0..r {
            let growth = self.eigenvalues[j].powi(k as i32);
            let coeff = self.amplitudes[j] * growth;
            for i in 0..n {
                x[i] += self.modes[[i, j]] * coeff;
            }
        }
        x.mapv(|z| z.re)
    }

    /// Continuous-time prediction at time `t` when `omega` is available:
    /// `x(t) ≈ Φ diag(e^{ω t}) b` (real part).
    ///
    /// Falls back to discrete prediction with `k = round(t / dt_hint)` if
    /// `omega` is `None` and `dt_hint` is provided.
    pub fn predict_continuous(&self, t: f64) -> Option<Array1<f64>> {
        let omega = self.omega.as_ref()?;
        let r = self.rank;
        let n = self.modes.nrows();
        let mut x = Array1::<Complex64>::zeros(n);
        for j in 0..r {
            let growth = (omega[j] * t).exp();
            let coeff = self.amplitudes[j] * growth;
            for i in 0..n {
                x[i] += self.modes[[i, j]] * coeff;
            }
        }
        Some(x.mapv(|z| z.re))
    }

    /// Reconstruction error on training snapshots: mean `‖x_k − x̂_k‖₂ / ‖x_k‖₂`
    /// over columns of the full trajectory matrix `snapshots` (n × m).
    pub fn relative_reconstruction_error(&self, snapshots: &Array2<f64>) -> f64 {
        let m = snapshots.ncols();
        if m == 0 {
            return 0.0;
        }
        let mut acc = 0.0;
        let mut count = 0usize;
        for k in 0..m {
            let x = snapshots.column(k);
            let xhat = self.predict_discrete(k);
            let denom = x.dot(&x).sqrt().max(1e-15);
            let diff = &xhat - &x.to_owned();
            acc += diff.dot(&diff).sqrt() / denom;
            count += 1;
        }
        acc / count as f64
    }
}

/// Options for [`dmd`].
#[derive(Debug, Clone)]
pub struct DmdConfig {
    /// Truncation rank. `None` keeps all singular values above `svd_tol`.
    pub rank: Option<usize>,
    /// Singular-value relative tolerance when `rank` is `None`
    /// (keep σᵢ > tol · σ_max).
    pub svd_tol: f64,
    /// Sample time Δt for continuous-time eigenvalues `ω = log(λ)/Δt`.
    /// `None` skips computing `omega`.
    pub dt: Option<f64>,
}

impl Default for DmdConfig {
    fn default() -> Self {
        Self {
            rank: None,
            svd_tol: 1e-10,
            dt: None,
        }
    }
}

/// Exact DMD from a full snapshot matrix.
///
/// # Arguments
/// * `snapshots` — state dimension × time (`n × m`), columns are consecutive states
/// * `config` — rank truncation and optional `dt`
///
/// Requires `m ≥ 2`.
pub fn dmd(snapshots: &Array2<f64>, config: &DmdConfig) -> DmdResult {
    let m = snapshots.ncols();
    assert!(m >= 2, "need at least 2 snapshots for DMD");
    let x = snapshots.slice(s![.., 0..m - 1]).to_owned();
    let x_prime = snapshots.slice(s![.., 1..m]).to_owned();
    dmd_pair(&x, &x_prime, config)
}

/// Exact DMD from paired snapshot matrices `X` and `X′` (same shape `n × (m−1)`).
pub fn dmd_pair(x: &Array2<f64>, x_prime: &Array2<f64>, config: &DmdConfig) -> DmdResult {
    assert_eq!(x.nrows(), x_prime.nrows(), "state dimensions must match");
    assert_eq!(x.ncols(), x_prime.ncols(), "snapshot counts must match");
    assert!(x.ncols() >= 1, "need at least one column pair");

    let n = x.nrows();
    let svd = Svd::compute(x);
    let singular_values = svd.s.clone();

    let r = resolve_rank(&svd.s, config);
    assert!(r > 0, "DMD rank resolved to 0; check data or svd_tol");

    let (u_r, s_r, vt_r) = svd.truncate(r);
    // V_r is n_snap × r  (vt is r × n_snap)
    let v_r = vt_r.t().to_owned();

    // Σ_r^{-1}
    let mut s_inv = Array2::<f64>::zeros((r, r));
    for i in 0..r {
        s_inv[[i, i]] = 1.0 / s_r[i].max(1e-15);
    }

    // Ã = U* X' V Σ^{-1}
    let a_tilde = u_r.t().dot(x_prime).dot(&v_r).dot(&s_inv);

    // Eigendecomposition of Ã (complex eigenvalues / eigenvectors)
    let (evals, evecs) = a_tilde
        .eig()
        .expect("eigendecomposition of reduced DMD operator failed");

    // Φ = X' V Σ^{-1} W  (exact DMD modes)
    let w = evecs; // Array2<Complex64>
    let v_sinv = v_r.dot(&s_inv);
    let v_sinv_c = real_matrix_to_complex(&v_sinv);
    let x_prime_c = real_matrix_to_complex(x_prime);
    let modes = x_prime_c.dot(&v_sinv_c).dot(&w);

    // Amplitudes from first snapshot of X: x₀ ≈ Φ b
    let x0 = x.column(0).to_owned();
    let amplitudes = fit_amplitudes(&modes, &x0);

    let eigenvalues = evals;

    let omega = config.dt.map(|dt| {
        assert!(dt > 0.0, "dt must be positive");
        eigenvalues.mapv(|lam| lam.ln() / dt)
    });

    let _ = n;

    DmdResult {
        eigenvalues,
        modes,
        amplitudes,
        singular_values,
        rank: r,
        omega,
    }
}

/// Build a snapshot matrix (`n × m`) from a sequence of state vectors.
pub fn snapshots_from_states(states: &[Array1<f64>]) -> Array2<f64> {
    assert!(!states.is_empty(), "states must not be empty");
    let n = states[0].len();
    let m = states.len();
    let mut out = Array2::zeros((n, m));
    for (k, s) in states.iter().enumerate() {
        assert_eq!(s.len(), n, "all states must share the same dimension");
        out.column_mut(k).assign(s);
    }
    out
}

/// Stack delay-embedded vectors (from [`crate::edim`]) as snapshot columns.
///
/// Input `embedded` is a list of delay vectors (each length `m_embed`);
/// output is `m_embed × n_vectors`.
pub fn snapshots_from_embedding(embedded: &[Vec<f64>]) -> Array2<f64> {
    assert!(!embedded.is_empty(), "embedded trajectories must not be empty");
    let n = embedded[0].len();
    let m = embedded.len();
    let mut out = Array2::zeros((n, m));
    for (k, v) in embedded.iter().enumerate() {
        assert_eq!(v.len(), n);
        for i in 0..n {
            out[[i, k]] = v[i];
        }
    }
    out
}

fn resolve_rank(s: &Array1<f64>, config: &DmdConfig) -> usize {
    if s.is_empty() {
        return 0;
    }
    let max_r = s.len();
    if let Some(r) = config.rank {
        return r.min(max_r).max(1);
    }
    let smax = s[0].abs().max(1e-15);
    let mut r = 0usize;
    for &si in s.iter() {
        if si.abs() > config.svd_tol * smax {
            r += 1;
        } else {
            break;
        }
    }
    r.max(1).min(max_r)
}

fn real_matrix_to_complex(a: &Array2<f64>) -> Array2<Complex64> {
    Array2::from_shape_fn(a.dim(), |(i, j)| Complex64::new(a[[i, j]], 0.0))
}

fn fit_amplitudes(modes: &Array2<Complex64>, x0: &Array1<f64>) -> Array1<Complex64> {
    // Solve Φ b ≈ x0 in least squares (complex).
    // Split into real system: for complex LS use real block form, or use
    // ndarray least squares on real/imag stacked when modes are nearly real.
    // Prefer: use least squares on complex via real 2n system if needed.
    //
    // Simple approach: use Moore-Penrose via SVD of real/imag block.
    let n = modes.nrows();
    let r = modes.ncols();
    // Real LS: [ReΦ; ImΦ] [Re b; Im b] is wrong for complex multiply.
    // Correct real form for Φ b = x0 with complex Φ, b:
    // [ ReΦ  -ImΦ ] [ Re b ]   [ Re x0 ]
    // [ ImΦ   ReΦ ] [ Im b ] = [ Im x0 ]
    let mut a = Array2::<f64>::zeros((2 * n, 2 * r));
    let mut rhs = Array1::<f64>::zeros(2 * n);
    for i in 0..n {
        rhs[i] = x0[i];
        rhs[n + i] = 0.0;
        for j in 0..r {
            let z = modes[[i, j]];
            a[[i, j]] = z.re;
            a[[i, r + j]] = -z.im;
            a[[n + i, j]] = z.im;
            a[[n + i, r + j]] = z.re;
        }
    }
    let sol = a.least_squares(&rhs).expect("amplitude least-squares failed").solution;
    let mut b = Array1::<Complex64>::zeros(r);
    for j in 0..r {
        b[j] = Complex64::new(sol[j], sol[r + j]);
    }
    b
}

// ——— Tests ———

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    /// Linear system x_{k+1} = A x_k with known eigenvalues.
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
    fn test_dmd_recovers_linear_eigenvalues() {
        // Rotation-scaling in 2D: eigenvalues of A are complex conjugates
        // A = [[0.9, -0.2], [0.2, 0.9]] → λ = 0.9 ± 0.2i
        let a = array![[0.9, -0.2], [0.2, 0.9]];
        let snaps = simulate_linear(&a, array![1.0, 0.0], 30);

        let result = dmd(
            &snaps,
            &DmdConfig {
                rank: Some(2),
                ..Default::default()
            },
        );

        assert_eq!(result.rank, 2);
        // Expected eigenvalues
        let expected = [Complex64::new(0.9, 0.2), Complex64::new(0.9, -0.2)];
        // Match each expected to nearest computed (order may vary)
        for exp in &expected {
            let min_dist = result
                .eigenvalues
                .iter()
                .map(|lam| (lam - exp).norm())
                .fold(f64::INFINITY, f64::min);
            assert!(
                min_dist < 1e-6,
                "eigenvalue {exp} not recovered; got {:?}",
                result.eigenvalues
            );
        }
    }

    #[test]
    fn test_dmd_prediction_on_linear_system() {
        let a = array![[0.95, 0.05], [0.0, 0.9]];
        let snaps = simulate_linear(&a, array![1.0, 1.0], 20);
        let result = dmd(
            &snaps,
            &DmdConfig {
                rank: Some(2),
                ..Default::default()
            },
        );

        // Predict a few steps and compare to true snapshots
        for k in [0usize, 5, 10, 15] {
            let pred = result.predict_discrete(k);
            let true_x = snaps.column(k);
            let err = (&pred - &true_x.to_owned()).mapv(|v| v * v).sum().sqrt();
            assert!(err < 1e-4, "step {k} prediction error {err}");
        }
    }

    #[test]
    fn test_dmd_reconstruction_error_small() {
        let a = array![[0.8, 0.1], [0.0, 0.7]];
        let snaps = simulate_linear(&a, array![2.0, -1.0], 15);
        let result = dmd(
            &snaps,
            &DmdConfig {
                rank: Some(2),
                ..Default::default()
            },
        );
        let err = result.relative_reconstruction_error(&snaps);
        assert!(err < 1e-4, "mean relative error {err}");
    }

    #[test]
    fn test_continuous_eigenvalues() {
        let a = array![[0.9, 0.0], [0.0, 0.8]];
        let snaps = simulate_linear(&a, array![1.0, 1.0], 10);
        let dt = 0.1;
        let result = dmd(
            &snaps,
            &DmdConfig {
                rank: Some(2),
                dt: Some(dt),
                ..Default::default()
            },
        );
        let omega = result.omega.as_ref().unwrap();
        // λ = e^{ω Δt} ⇒ ω = log(λ)/Δt; for real λ>0, ω real
        for j in 0..2 {
            let lam = result.eigenvalues[j];
            let w = omega[j];
            let rel = (w * dt).exp();
            assert!((rel - lam).norm() < 1e-8);
        }
    }

    #[test]
    fn test_snapshots_from_states() {
        let states = vec![array![1.0, 2.0], array![3.0, 4.0]];
        let s = snapshots_from_states(&states);
        assert_eq!(s.shape(), &[2, 2]);
        assert!((s[[0, 1]] - 3.0).abs() < 1e-15);
    }

    #[test]
    fn test_snapshots_from_embedding() {
        let emb = vec![vec![1.0, 2.0, 3.0], vec![2.0, 3.0, 4.0]];
        let s = snapshots_from_embedding(&emb);
        assert_eq!(s.shape(), &[3, 2]);
        assert!((s[[2, 1]] - 4.0).abs() < 1e-15);
    }

    #[test]
    fn test_rank_auto() {
        let a = array![[0.5, 0.0], [0.0, 0.4]];
        let snaps = simulate_linear(&a, array![1.0, 1.0], 12);
        let result = dmd(&snaps, &DmdConfig::default());
        assert!(result.rank >= 1);
        assert!(result.rank <= 2);
    }
}
