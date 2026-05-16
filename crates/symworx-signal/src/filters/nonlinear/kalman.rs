// filters/nonlinear/kalman.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use ndarray::{array, Array1, Array2};
use ndarray_linalg::Inverse;

// ==========================================================
// Kalman Filter
// ==========================================================
#[derive(Debug, Clone)]
pub struct KalmanFilter {
    x: Array1<f64>,   // state (2,)
    p: Array2<f64>,   // covariance (2,2)
    f: Array2<f64>,   // state transition (2,2)
    q: Array2<f64>,   // process noise (2,2)
    h: Array2<f64>,   // measurement matrix (1,2)
    r: Array2<f64>,   // measurement noise (1,1)
}

impl KalmanFilter {
    pub fn new(dt: f64, process_var: f64, meas_var: f64) -> Self {
        let f = array![
            [1.0, dt],
            [0.0, 1.0]
        ];

        let q = array![
            [process_var * dt.powi(4) / 4.0, process_var * dt.powi(3) / 2.0],
            [process_var * dt.powi(3) / 2.0, process_var * dt.powi(2)]
        ];

        let h = array![[1.0, 0.0]]; // (1×2)

        let r = array![[meas_var]]; // (1×1)

        let x = array![0.0, 0.0];
        let p = Array2::eye(2) * 1e3;

        Self { x, p, f, q, h, r }
    }

    pub fn predict(&mut self) {
        // x = F x
        self.x = self.f.dot(&self.x);

        // P = F P F^T + Q
        self.p = self.f.dot(&self.p).dot(&self.f.t()) + &self.q;
    }

    pub fn update(&mut self, z: f64) {
        let z_vec = array![[z]]; // (1×1)

        // y = z - H x
        let hx = self.h.dot(&self.x); // (1,)
        let y = &z_vec - &hx.insert_axis(ndarray::Axis(0));

        // S = H P H^T + R
        let s = self.h.dot(&self.p).dot(&self.h.t()) + &self.r;

        // K = P H^T S^{-1}
        let k = self.p.dot(&self.h.t()).dot(&s.inv().unwrap()); // (2×1)

        // x = x + K y
        let y_scalar = y[[0, 0]];
        self.x = &self.x + &(&k.column(0) * y_scalar);

        // P = (I - K H) P
        let i = Array2::eye(2);
        self.p = (i - k.dot(&self.h)).dot(&self.p);
    }

    pub fn state(&self) -> (f64, f64) {
        (self.x[0], self.x[1])
    }
}


// ==========================================================
// TESTS
// ==========================================================
#[cfg(test)]
mod test_kalman {
    use super::*;

    #[test]
    fn test_kalman_filter() {
        let dt = 1.0;
        let process_var = 1e-5;
        let meas_var = 0.1;
        let mut kf = KalmanFilter::new(dt, process_var, meas_var);

        // Simulate measurements of a constant velocity (position = velocity * time)
        let true_velocity = 1.0; // m/s
        let mut measurements = Vec::new();
        for t in 0..10 {
            let true_position = true_velocity * (t as f64);
            let noisy_measurement = true_position + (rand::random::<f64>() - 0.5) * 0.2; // Add noise
            measurements.push(noisy_measurement);
        }

        for z in measurements {
            kf.predict();
            kf.update(z);
            let (pos, vel) = kf.state();
            println!("Measurement: {:.3}, Estimated Position: {:.3}, Estimated Velocity: {:.3}", z, pos, vel);
        }
    }
}
