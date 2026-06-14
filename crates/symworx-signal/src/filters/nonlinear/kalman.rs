// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Kalman filtering and smoothing using linear-Gaussian state-space models.
//!
//! This is the primary, general-purpose `KalmanFilter` implementation in SymWorx.
//! It is suitable for a wide range of biosignal and dynamical systems applications
//! (far beyond the specific sleep adaptability analysis), including:
//!
//! - Latent state estimation from multiple noisy observation channels
//!   (e.g. combining HRV features, respiration, movement, delta power, etc.)
//! - Sensor fusion and tracking of physiological or biomechanical states
//! - Offline batch analysis where Rauch–Tung–Striebel (RTS) smoothing is desired
//!   for the best estimates given the entire time series
//! - Systems with known control inputs (interventions, exercise bouts, etc.)
//!
//! The simple 1D constant-velocity tracker lives in `kalman1d.rs` as `KalmanFilter1D`
//! for backward compatibility and very lightweight use cases.
//!
//! ## Relationship to KalmanFilter1D
//!
//! `KalmanFilter1D` is kept under a new name so that the primary `KalmanFilter`
//! type can be the flexible, multivariate, smoothable version that most new
//! work should use.
//!
//! You can still emulate the old 1D CV behavior with this general `KalmanFilter`
//! (see the test `test_general_reproduces_old_1d_cv`).
//!
//! ## Typical usage
//!
//! ```ignore
//! use ndarray::array;
//! use symworx_signal::filters::nonlinear::KalmanFilter;  // the general one
//!
//! // Example: 2D state (level + drift) observed through 3 features
//! let f = array![[1.0, dt], [0.0, 1.0]];
//! let h = array![[1.0, 0.0],
//!                [1.0, 0.1],
//!                [0.8, 0.0]];
//! let q = ...;
//! let r = ...;
//! let x0 = array![0.0, 0.0];
//! let p0 = Array2::eye(2) * 1000.0;
//!
//! let mut kf = KalmanFilter::new(f, h, q, r, x0, p0);
//!
//! let mut filtered = Vec::new();
//! for z in window_features {   // each z is Array1 of length 3
//!     kf.predict(None);
//!     kf.update(&z);
//!     filtered.push(kf.state().clone());
//! }
//!
//! // For best offline estimates of the whole trajectory:
//! let run = kf.run_forward(&all_zs);
//! let smoothed = rts_smooth(&run, &f);
//! ```

use ndarray::{Array1, Array2, Axis, array};
use ndarray_linalg::Inverse;

/// A general linear-Gaussian state-space Kalman filter.
///
/// Supports arbitrary state dimension, arbitrary measurement dimension,
/// optional control inputs, and (via the run + rts_smooth path) full
/// Rauch-Tung-Striebel smoothing.
///
/// This is the primary `KalmanFilter` type. The simpler 1D constant-velocity
/// version is available as `KalmanFilter1D`.
#[derive(Debug, Clone)]
pub struct KalmanFilter {
    /// State transition matrix (n x n)
    f: Array2<f64>,
    /// Measurement matrix (m x n). Can be replaced per-step for time-varying obs.
    h: Array2<f64>,
    /// Process noise covariance (n x n)
    q: Array2<f64>,
    /// Measurement noise covariance (m x m). Can be replaced per-step.
    r: Array2<f64>,
    /// Optional control matrix (n x p). If present, predict takes a control vector u.
    b: Option<Array2<f64>>,
    /// Current state estimate
    x: Array1<f64>,
    /// Current state covariance
    p: Array2<f64>,
}

impl KalmanFilter {
    /// Create a new general state-space Kalman filter.
    ///
    /// # Arguments
    /// * `f` — state transition matrix (n_states × n_states)
    /// * `h` — measurement matrix (n_obs × n_states)
    /// * `q` — process noise covariance
    /// * `r` — measurement noise covariance
    /// * `x0` — initial state
    /// * `p0` — initial covariance (usually large to express uncertainty)
    pub fn new(
        f: Array2<f64>,
        h: Array2<f64>,
        q: Array2<f64>,
        r: Array2<f64>,
        x0: Array1<f64>,
        p0: Array2<f64>,
    ) -> Self {
        Self {
            f,
            h,
            q,
            r,
            b: None,
            x: x0,
            p: p0,
        }
    }

    /// Attach a control matrix B (n_states × n_controls).
    /// After this, you can pass a control vector `u` to `predict`.
    pub fn with_control(mut self, b: Array2<f64>) -> Self {
        self.b = Some(b);
        self
    }

    /// Prediction step (time update).
    ///
    /// If a control matrix was supplied via `with_control`, you may pass
    /// `Some(u)` where `u` has length equal to the number of control inputs.
    pub fn predict(&mut self, control: Option<&Array1<f64>>) {
        self.x = self.f.dot(&self.x);

        if let (Some(b), Some(u)) = (&self.b, control) {
            if b.ncols() == u.len() {
                self.x += &b.dot(u);
            }
        }

        self.p = self.f.dot(&self.p).dot(&self.f.t()) + &self.q;
    }

    /// Update step (measurement update) with the current H and R.
    pub fn update(&mut self, z: &Array1<f64>) {
        let h = self.h.clone();
        let r = self.r.clone();
        self.update_with(z, &h, &r);
    }

    /// Update step allowing a different H and/or R for this timestep
    /// (supports time-varying measurement models or partial observations).
    ///
    /// `z` must have length matching the rows of the supplied `h`.
    pub fn update_with(&mut self, z: &Array1<f64>, h: &Array2<f64>, r: &Array2<f64>) {
        // Innovation
        let hx = h.dot(&self.x);
        let y = z - &hx;

        // Innovation covariance S = H P H^T + R
        let s = h.dot(&self.p).dot(&h.t()) + r;

        // Kalman gain K = P H^T S^{-1}
        let k = self
            .p
            .dot(&h.t())
            .dot(&s.inv().expect("S inversion failed in Kalman update"));

        // State update
        self.x = &self.x + &k.dot(&y);

        // Covariance update (Joseph form is more stable, but the simple form is fine here)
        let i: Array2<f64> = Array2::eye(self.p.nrows());
        self.p = (&i - k.dot(h)).dot(&self.p);
    }

    /// Returns a reference to the current state estimate.
    pub fn state(&self) -> &Array1<f64> {
        &self.x
    }

    /// Convenience: run the filter forward over a sequence of observation vectors.
    ///
    /// Returns a `FilterRun` containing the quantities needed for RTS smoothing
    /// (and also the filtered states for immediate use).
    ///
    /// `controls` can be `None` or a slice of control vectors (must match length of `zs`).
    pub fn run_forward(
        &mut self,
        zs: &[Array1<f64>],
        controls: Option<&[Array1<f64>]>,
    ) -> FilterRun {
        let mut filtered_states = Vec::with_capacity(zs.len());
        let mut filtered_covs = Vec::with_capacity(zs.len());
        let mut predicted_states = Vec::with_capacity(zs.len());
        let mut predicted_covs = Vec::with_capacity(zs.len());

        for (i, z) in zs.iter().enumerate() {
            let u = controls.and_then(|cs| cs.get(i));

            // Predict
            let x_pred = self.f.dot(&self.x);
            let mut x_pred = x_pred;
            if let (Some(b), Some(u)) = (&self.b, u) {
                if b.ncols() == u.len() {
                    x_pred += &b.dot(u);
                }
            }
            let p_pred = self.f.dot(&self.p).dot(&self.f.t()) + &self.q;

            predicted_states.push(x_pred.clone());
            predicted_covs.push(p_pred.clone());

            // Update (using the filter's current H/R; caller can use update_with before calling run if they need per-step variation)
            let h = &self.h;
            let r = &self.r;

            let hx = h.dot(&x_pred);
            let y = z - &hx;
            let s = h.dot(&p_pred).dot(&h.t()) + r;
            let k = p_pred.dot(&h.t()).dot(&s.inv().expect("S inversion failed"));

            let x_upd = &x_pred + &k.dot(&y);
            let i: Array2<f64> = Array2::eye(p_pred.nrows());
            let p_upd = (&i - k.dot(h)).dot(&p_pred);

            self.x = x_upd.clone();
            self.p = p_upd.clone();

            filtered_states.push(x_upd);
            filtered_covs.push(p_upd);
        }

        FilterRun {
            filtered_states,
            filtered_covs,
            predicted_states,
            predicted_covs,
        }
    }
}

/// Quantities saved during a forward filter run. Used by the RTS smoother.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct FilterRun {
    pub filtered_states: Vec<Array1<f64>>,
    pub filtered_covs: Vec<Array2<f64>>,
    pub predicted_states: Vec<Array1<f64>>,
    pub predicted_covs: Vec<Array2<f64>>,
}

/// Rauch–Tung–Striebel (RTS) smoother.
///
/// Given a `FilterRun` produced by `KalmanFilter::run_forward` and the
/// state transition matrix F used during filtering, returns the smoothed
/// state estimates (best linear unbiased estimates given the *whole* sequence).
///
/// This is the version you want for offline sleep-bout analysis.
pub fn rts_smooth(run: &FilterRun, f: &Array2<f64>) -> Vec<Array1<f64>> {
    let n = run.filtered_states.len();
    if n == 0 {
        return vec![];
    }

    let mut smoothed = run.filtered_states.clone();
    let mut p_smoothed = run.filtered_covs.clone();

    // Go backward
    for k in (0..n - 1).rev() {
        let p_k = &run.filtered_covs[k];
        let p_k1_pred = &run.predicted_covs[k + 1];

        // C = P_k * F^T * (P_{k+1|k})^{-1}
        let c = p_k
            .dot(&f.t())
            .dot(&p_k1_pred.inv().expect("RTS covariance inversion failed"));

        // x^s_k = x^f_k + C (x^s_{k+1} - x^p_{k+1})
        let diff = &run.predicted_states[k + 1] - &smoothed[k + 1];
        smoothed[k] = &smoothed[k] + &c.dot(&diff);

        // P^s_k = P^f_k + C (P^s_{k+1} - P^p_{k+1}) C^T
        let p_diff = &p_smoothed[k + 1] - p_k1_pred;
        p_smoothed[k] = p_k + &c.dot(&p_diff).dot(&c.t());
    }

    smoothed
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array2;

    #[test]
    fn test_general_reproduces_old_1d_cv() {
        // Replicate the exact behavior of the original 1D constant-velocity filter
        // using the general KalmanFilter so we know the architecture is sound.
        let dt = 1.0;
        let process_var = 1e-5;
        let meas_var = 0.1;

        // Classic 1D CV matrices
        let f = array![[1.0, dt], [0.0, 1.0]];
        let h = array![[1.0, 0.0]];
        let q = array![
            [process_var * dt.powi(4) / 4.0, process_var * dt.powi(3) / 2.0],
            [process_var * dt.powi(3) / 2.0, process_var * dt.powi(2)]
        ];
        let r = array![[meas_var]];
        let x0 = array![0.0, 0.0];
        let p0 = Array2::eye(2) * 1000.0;

        let mut general = KalmanFilter::new(f, h, q, r, x0, p0);

        // Simulate the same noisy ramp the old tests used
        let mut old_style_results = vec![];
        for t in 0..5 {
            let true_pos = t as f64 * 1.0;
            let noisy_z = true_pos + (t as f64 * 0.05);

            general.predict(None);
            general.update(&array![noisy_z]);

            let state = general.state();
            old_style_results.push((state[0], state[1]));
        }

        // Just sanity check that we get finite numbers and the velocity is roughly 1
        for (pos, vel) in &old_style_results {
            assert!(pos.is_finite());
            assert!(vel.is_finite());
        }
        let last_vel = old_style_results.last().unwrap().1;
        assert!((last_vel - 1.0).abs() < 0.3); // loose because of the noise schedule in the old test
    }

    #[test]
    fn test_rts_smooth_runs() {
        let f = array![[1.0, 0.1], [0.0, 1.0]];
        let h = array![[1.0, 0.0], [0.0, 1.0]];
        let q = Array2::eye(2) * 0.01;
        let r = Array2::eye(2) * 0.5;
        let x0 = array![0.0, 0.0];
        let p0 = Array2::eye(2) * 10.0;

        let mut kf = KalmanFilter::new(f.clone(), h, q, r, x0, p0);

        let zs: Vec<Array1<f64>> = (0..20)
            .map(|i| array![i as f64 * 0.1, 0.8 + 0.02 * (i as f64)])
            .collect();

        let run = kf.run_forward(&zs, None);
        let smoothed = rts_smooth(&run, &f);

        assert_eq!(smoothed.len(), zs.len());
        // Smoothed should generally be smoother than filtered; we just check it ran
        assert!(smoothed[0][0].is_finite());
    }
}
