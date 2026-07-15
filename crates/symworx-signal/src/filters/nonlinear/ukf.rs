// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Unscented Kalman Filter (UKF).
//!
//! Propagates **sigma points** through nonlinear `f` and `h` (Julier & Uhlmann;
//! Wan & van der Merwe) without analytic Jacobians.
//!
//! ```text
//! χ = unscented(x, P)
//! χ⁺ = f(χ),   x⁻ = Σ w_m χ⁺
//! P⁻ = Σ w_c (χ⁺ − x⁻)(·)ᵀ + Q
//! γ = h(χ⁺),   ẑ = Σ w_m γ
//! update with cross-covariance P_xz and innovation covariance
//! ```

use ndarray::{
    Array1,
    Array2,
};
use ndarray_linalg::{
    Cholesky,
    Inverse,
    UPLO,
};

/// UKF scaling / weight parameters (standard additive form).
#[derive(Debug, Clone)]
pub struct UkfParams {
    /// Spread of sigma points (`α ∈ (0, 1]`, typically small e.g. `1e-3`).
    pub alpha: f64,
    /// Prior knowledge of distribution (`β = 2` optimal for Gaussians).
    pub beta: f64,
    /// Secondary scaling (`κ` usually `0` or `3 − n`).
    pub kappa: f64,
}

impl Default for UkfParams {
    fn default() -> Self {
        Self {
            alpha: 1e-3,
            beta: 2.0,
            kappa: 0.0,
        }
    }
}

/// Unscented Kalman filter.
#[derive(Debug, Clone)]
pub struct UnscentedKalmanFilter {
    /// Process noise Q.
    pub q: Array2<f64>,
    /// Measurement noise R.
    pub r: Array2<f64>,
    /// UKF parameters.
    pub params: UkfParams,
    x: Array1<f64>,
    p: Array2<f64>,
}

impl UnscentedKalmanFilter {
    /// Create a UKF with default Julier parameters.
    pub fn new(x0: Array1<f64>, p0: Array2<f64>, q: Array2<f64>, r: Array2<f64>) -> Self {
        Self::with_params(x0, p0, q, r, UkfParams::default())
    }

    /// Create a UKF with custom sigma-point parameters.
    pub fn with_params(
        x0: Array1<f64>,
        p0: Array2<f64>,
        q: Array2<f64>,
        r: Array2<f64>,
        params: UkfParams,
    ) -> Self {
        let n = x0.len();
        assert_eq!(p0.shape(), &[n, n]);
        assert_eq!(q.shape(), &[n, n]);
        assert_eq!(r.nrows(), r.ncols());
        Self {
            q,
            r,
            params,
            x: x0,
            p: p0,
        }
    }

    /// Current state.
    pub fn state(&self) -> &Array1<f64> {
        &self.x
    }

    /// Current covariance.
    pub fn covariance(&self) -> &Array2<f64> {
        &self.p
    }

    /// Predict through nonlinear process `f(x, u) → x_next`.
    pub fn predict<F>(&mut self, f: F, u: Option<&Array1<f64>>)
    where
        F: Fn(&Array1<f64>, Option<&Array1<f64>>) -> Array1<f64>,
    {
        let n = self.x.len();
        let (sigmas, w_m, w_c) = sigma_points(&self.x, &self.p, &self.params);

        // Propagate
        let mut sigmas_f = Array2::zeros((n, 2 * n + 1));
        for i in 0..2 * n + 1 {
            let xi = sigmas.column(i).to_owned();
            sigmas_f.column_mut(i).assign(&f(&xi, u));
        }

        // Predicted mean
        let mut x_pred = Array1::zeros(n);
        for i in 0..2 * n + 1 {
            x_pred = x_pred + w_m[i] * &sigmas_f.column(i).to_owned();
        }

        // Predicted covariance
        let mut p_pred = self.q.clone();
        for i in 0..2 * n + 1 {
            let d = &sigmas_f.column(i).to_owned() - &x_pred;
            p_pred = p_pred + w_c[i] * outer(&d, &d);
        }

        self.x = x_pred;
        self.p = p_pred;
        // Store propagated sigmas for update efficiency? Recompute in update from x,P
        // (standard: re-draw sigma points at predicted mean for measurement)
        let _ = sigmas_f;
    }

    /// Measurement update through nonlinear `h(x) → z`.
    pub fn update<H>(&mut self, z: &Array1<f64>, h: H)
    where
        H: Fn(&Array1<f64>) -> Array1<f64>,
    {
        let n = self.x.len();
        let m = z.len();
        assert_eq!(self.r.nrows(), m);

        let (sigmas, w_m, w_c) = sigma_points(&self.x, &self.p, &self.params);

        // Measurement sigma points
        let mut z_sig = Array2::zeros((m, 2 * n + 1));
        for i in 0..2 * n + 1 {
            let xi = sigmas.column(i).to_owned();
            let zi = h(&xi);
            assert_eq!(zi.len(), m);
            z_sig.column_mut(i).assign(&zi);
        }

        let mut z_mean: Array1<f64> = Array1::zeros(m);
        for i in 0..2 * n + 1 {
            z_mean = z_mean + w_m[i] * &z_sig.column(i).to_owned();
        }

        let mut p_zz = self.r.clone();
        let mut p_xz: Array2<f64> = Array2::zeros((n, m));
        for i in 0..2 * n + 1 {
            let dz = &z_sig.column(i).to_owned() - &z_mean;
            let dx = &sigmas.column(i).to_owned() - &self.x;
            p_zz = p_zz + w_c[i] * outer(&dz, &dz);
            p_xz = p_xz + w_c[i] * outer(&dx, &dz);
        }

        let k = p_xz.dot(&p_zz.inv().expect("UKF P_zz inversion failed"));
        let innov = z - &z_mean;
        self.x = &self.x + &k.dot(&innov);
        self.p = &self.p - &k.dot(&p_zz).dot(&k.t());
    }
}

/// Generate `2n+1` sigma points (columns) and mean/covariance weights.
pub fn sigma_points(
    x: &Array1<f64>,
    p: &Array2<f64>,
    params: &UkfParams,
) -> (Array2<f64>, Array1<f64>, Array1<f64>) {
    let n = x.len();
    let lambda = params.alpha.powi(2) * (n as f64 + params.kappa) - n as f64;
    let c = n as f64 + lambda;

    // √((n+λ) P) via Cholesky
    let scaled = p * c;
    let chol = scaled
        .cholesky(UPLO::Lower)
        .expect("UKF Cholesky failed — P may not be SPD; add jitter");

    let n_sig = 2 * n + 1;
    let mut sigmas = Array2::zeros((n, n_sig));
    sigmas.column_mut(0).assign(x);
    for i in 0..n {
        let col = chol.column(i);
        sigmas.column_mut(i + 1).assign(&(x + &col.to_owned()));
        sigmas.column_mut(i + 1 + n).assign(&(x - &col.to_owned()));
    }

    let mut w_m = Array1::zeros(n_sig);
    let mut w_c = Array1::zeros(n_sig);
    w_m[0] = lambda / c;
    w_c[0] = lambda / c + (1.0 - params.alpha.powi(2) + params.beta);
    let wi = 1.0 / (2.0 * c);
    for i in 1..n_sig {
        w_m[i] = wi;
        w_c[i] = wi;
    }

    (sigmas, w_m, w_c)
}

fn outer(a: &Array1<f64>, b: &Array1<f64>) -> Array2<f64> {
    let mut out = Array2::zeros((a.len(), b.len()));
    for i in 0..a.len() {
        for j in 0..b.len() {
            out[[i, j]] = a[i] * b[j];
        }
    }
    out
}

// ——— Tests ———

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    #[test]
    fn test_sigma_points_mean() {
        let x = array![1.0, 2.0];
        let p = Array2::eye(2);
        let (sig, w_m, _) = sigma_points(&x, &p, &UkfParams::default());
        let mut mean: Array1<f64> = Array1::zeros(2);
        for i in 0..sig.ncols() {
            mean = mean + w_m[i] * &sig.column(i).to_owned();
        }
        assert!((mean[0] - 1.0).abs() < 1e-9);
        assert!((mean[1] - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_ukf_linear_tracking() {
        let mut ukf =
            UnscentedKalmanFilter::new(array![0.0], array![[1.0]], array![[0.01]], array![[0.1]]);
        let f = |x: &Array1<f64>, _: Option<&Array1<f64>>| array![0.9 * x[0]];
        let h = |x: &Array1<f64>| array![x[0]];

        let mut true_x = 1.0_f64;
        for _ in 0..25 {
            true_x *= 0.9;
            ukf.predict(&f, None);
            ukf.update(&array![true_x], &h);
        }
        assert!(
            (ukf.state()[0] - true_x).abs() < 0.2,
            "est {} true {true_x}",
            ukf.state()[0]
        );
    }

    #[test]
    fn test_ukf_nonlinear_measurement() {
        // Estimate θ from sin(θ) measurements
        let mut ukf = UnscentedKalmanFilter::with_params(
            array![0.0],
            array![[1.0]],
            array![[1e-5]],
            array![[0.01]],
            UkfParams {
                alpha: 0.1,
                beta: 2.0,
                kappa: 0.0,
            },
        );
        let f = |x: &Array1<f64>, _: Option<&Array1<f64>>| array![x[0]];
        let h = |x: &Array1<f64>| array![x[0].sin()];
        let true_theta = 0.4_f64;
        for _ in 0..40 {
            ukf.predict(&f, None);
            ukf.update(&array![true_theta.sin()], &h);
        }
        assert!(
            (ukf.state()[0] - true_theta).abs() < 0.15,
            "θ est {}",
            ukf.state()[0]
        );
    }
}
