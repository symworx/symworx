// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! KalmanFilter1D — a simple 1D constant-velocity Kalman filter.
//!
//! This is the legacy/simple 1D tracker (previously named `KalmanFilter`).
//! For general multivariate state estimation, control inputs, and RTS smoothing,
//! use the primary `KalmanFilter` from the `state_space` module.

use ndarray::{
    Array1,
    Array2,
    Axis,
    array,
};
use ndarray_linalg::Inverse;

/// Simple 1D Kalman Filter (position + velocity state).
#[derive(Debug, Clone)]
pub struct KalmanFilter1D {
    /// State vector: [position, velocity]
    x: Array1<f64>,
    /// State covariance matrix
    p: Array2<f64>,
    /// State transition matrix
    f: Array2<f64>,
    /// Process noise covariance
    q: Array2<f64>,
    /// Measurement matrix
    h: Array2<f64>,
    /// Measurement noise covariance
    r: Array2<f64>,
}

impl KalmanFilter1D {
    /// Creates a new Kalman Filter for constant velocity tracking.
    ///
    /// # Arguments
    /// * `dt` — Time step (seconds)
    /// * `process_var` — Process noise variance
    /// * `meas_var` — Measurement noise variance
    pub fn new(dt: f64, process_var: f64, meas_var: f64) -> Self {
        let f = array![[1.0, dt], [0.0, 1.0]];

        let q = array![
            [
                process_var * dt.powi(4) / 4.0,
                process_var * dt.powi(3) / 2.0
            ],
            [process_var * dt.powi(3) / 2.0, process_var * dt.powi(2)]
        ];

        let h = array![[1.0, 0.0]];
        let r = array![[meas_var]];

        let x = array![0.0, 0.0];
        let p = Array2::eye(2) * 1000.0; // High initial uncertainty

        Self { x, p, f, q, h, r }
    }

    /// Prediction step (time update).
    pub fn predict(&mut self) {
        self.x = self.f.dot(&self.x);
        self.p = self.f.dot(&self.p).dot(&self.f.t()) + &self.q;
    }

    /// Update step (measurement update).
    pub fn update(&mut self, z: f64) {
        let z_vec = array![[z]];

        // Innovation (measurement residual)
        let hx = self.h.dot(&self.x);
        let y = z_vec - &hx.insert_axis(Axis(0));

        // Innovation covariance
        let s = self.h.dot(&self.p).dot(&self.h.t()) + &self.r;

        // Kalman gain
        let k: Array2<f64> = self
            .p
            .dot(&self.h.t())
            .dot(&s.inv().expect("Matrix inversion failed"));

        // State update: x = x + K * y
        let correction = k.dot(&y);
        self.x += &correction.column(0); // Extract as 1D vector

        // Covariance update: P = (I - K H) P
        let i = Array2::eye(2);
        self.p = (i - k.dot(&self.h)).dot(&self.p);
    }

    /// Returns current estimated (position, velocity).
    pub fn state(&self) -> (f64, f64) {
        (self.x[0], self.x[1])
    }
}

// ——— Tests ———

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_initialization() {
        let kf = KalmanFilter1D::new(0.1, 1e-4, 0.1);
        let (pos, vel) = kf.state();
        assert_eq!(pos, 0.0);
        assert_eq!(vel, 0.0);
    }

    #[test]
    fn test_kalman_predict_update() {
        let mut kf = KalmanFilter1D::new(1.0, 1e-5, 0.1);

        // Simulate constant velocity of 1.0
        for t in 0..5 {
            let true_pos = t as f64 * 1.0;
            let noisy_z = true_pos + (t as f64 * 0.05); // small noise

            kf.predict();
            kf.update(noisy_z);

            let (pos, vel) = kf.state();
            assert!(pos.is_finite());
            assert!(vel.is_finite());
        }
    }
}
