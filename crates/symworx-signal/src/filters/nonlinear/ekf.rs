// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Extended Kalman Filter (EKF) for nonlinear process / measurement models.
//!
//! Linearizes `f` and `h` via Jacobians at the current estimate:
//!
//! ```text
//! x⁻ = f(x, u)
//! P⁻ = F P Fᵀ + Q
//! y  = z − h(x⁻)
//! S  = H P⁻ Hᵀ + R
//! K  = P⁻ Hᵀ S⁻¹
//! x  = x⁻ + K y
//! P  = (I − K H) P⁻
//! ```
//!
//! Jacobians may be supplied analytically or approximated with central
//! finite differences ([`numerical_jacobian`]).

use ndarray::{
    Array1,
    Array2,
};
use ndarray_linalg::Inverse;

/// Extended Kalman filter with user-provided nonlinear models.
///
/// Process: `x⁺ = f(x, u)` (control optional).  
/// Measurement: `z = h(x) + v`.
#[derive(Debug, Clone)]
pub struct ExtendedKalmanFilter {
    /// Process noise covariance Q (n × n).
    pub q: Array2<f64>,
    /// Measurement noise covariance R (m × m).
    pub r: Array2<f64>,
    /// Current state estimate.
    x: Array1<f64>,
    /// Current covariance.
    p: Array2<f64>,
}

impl ExtendedKalmanFilter {
    /// Create an EKF with initial state / covariance and noise covariances.
    pub fn new(x0: Array1<f64>, p0: Array2<f64>, q: Array2<f64>, r: Array2<f64>) -> Self {
        let n = x0.len();
        assert_eq!(p0.nrows(), n);
        assert_eq!(p0.ncols(), n);
        assert_eq!(q.nrows(), n);
        assert_eq!(q.ncols(), n);
        assert_eq!(r.nrows(), r.ncols());
        Self {
            q,
            r,
            x: x0,
            p: p0,
        }
    }

    /// Current state estimate.
    pub fn state(&self) -> &Array1<f64> {
        &self.x
    }

    /// Current covariance.
    pub fn covariance(&self) -> &Array2<f64> {
        &self.p
    }

    /// Mutable access to state (for advanced resets).
    pub fn state_mut(&mut self) -> &mut Array1<f64> {
        &mut self.x
    }

    /// Predict with analytic process Jacobian `F = ∂f/∂x`.
    ///
    /// * `f` — `f(x, u_opt) → x_next`
    /// * `f_jacobian` — `F(x, u_opt) → n × n`
    /// * `u` — optional control
    pub fn predict<F, J>(&mut self, f: F, f_jacobian: J, u: Option<&Array1<f64>>)
    where
        F: Fn(&Array1<f64>, Option<&Array1<f64>>) -> Array1<f64>,
        J: Fn(&Array1<f64>, Option<&Array1<f64>>) -> Array2<f64>,
    {
        let f_mat = f_jacobian(&self.x, u);
        self.x = f(&self.x, u);
        self.p = f_mat.dot(&self.p).dot(&f_mat.t()) + &self.q;
    }

    /// Predict using finite-difference Jacobian of `f`.
    pub fn predict_fd<F>(&mut self, f: F, u: Option<&Array1<f64>>, eps: f64)
    where
        F: Fn(&Array1<f64>, Option<&Array1<f64>>) -> Array1<f64>,
    {
        let f_mat = numerical_jacobian(|x| f(x, u), &self.x, eps);
        self.x = f(&self.x, u);
        self.p = f_mat.dot(&self.p).dot(&f_mat.t()) + &self.q;
    }

    /// Update with analytic measurement Jacobian `H = ∂h/∂x`.
    pub fn update<H, J>(&mut self, z: &Array1<f64>, h: H, h_jacobian: J)
    where
        H: Fn(&Array1<f64>) -> Array1<f64>,
        J: Fn(&Array1<f64>) -> Array2<f64>,
    {
        let h_mat = h_jacobian(&self.x);
        let z_pred = h(&self.x);
        assert_eq!(z.len(), z_pred.len());
        assert_eq!(h_mat.nrows(), z.len());
        assert_eq!(h_mat.ncols(), self.x.len());

        let innov = z - &z_pred;
        let s = h_mat.dot(&self.p).dot(&h_mat.t()) + &self.r;
        let k = self
            .p
            .dot(&h_mat.t())
            .dot(&s.inv().expect("EKF innovation covariance inversion failed"));

        self.x = &self.x + &k.dot(&innov);
        let i: Array2<f64> = Array2::eye(self.x.len());
        self.p = (&i - k.dot(&h_mat)).dot(&self.p);
    }

    /// Update using finite-difference measurement Jacobian.
    pub fn update_fd<H>(&mut self, z: &Array1<f64>, h: H, eps: f64)
    where
        H: Fn(&Array1<f64>) -> Array1<f64>,
    {
        let h_mat = numerical_jacobian(&h, &self.x, eps);
        let z_pred = h(&self.x);
        let innov = z - &z_pred;
        let s = h_mat.dot(&self.p).dot(&h_mat.t()) + &self.r;
        let k = self
            .p
            .dot(&h_mat.t())
            .dot(&s.inv().expect("EKF innovation covariance inversion failed"));

        self.x = &self.x + &k.dot(&innov);
        let i: Array2<f64> = Array2::eye(self.x.len());
        self.p = (&i - k.dot(&h_mat)).dot(&self.p);
    }
}

/// Central finite-difference Jacobian of `g: R^n → R^m` at `x`.
///
/// `J[:, j] ≈ (g(x+ε e_j) − g(x−ε e_j)) / (2ε)`
pub fn numerical_jacobian<G>(g: G, x: &Array1<f64>, eps: f64) -> Array2<f64>
where
    G: Fn(&Array1<f64>) -> Array1<f64>,
{
    let n = x.len();
    let g0 = g(x);
    let m = g0.len();
    let mut jac = Array2::zeros((m, n));
    let mut x_pert = x.clone();

    for j in 0..n {
        let xj = x[j];
        x_pert[j] = xj + eps;
        let gp = g(&x_pert);
        x_pert[j] = xj - eps;
        let gm = g(&x_pert);
        x_pert[j] = xj;
        for i in 0..m {
            jac[[i, j]] = (gp[i] - gm[i]) / (2.0 * eps);
        }
    }
    jac
}

// ——— Tests ———

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    #[test]
    fn test_numerical_jacobian_linear() {
        // g(x) = A x
        let a = array![[2.0, 0.0], [0.0, 3.0], [1.0, 1.0]];
        let g = |x: &Array1<f64>| a.dot(x);
        let j = numerical_jacobian(g, &array![1.0, 2.0], 1e-6);
        let err = (&j - &a).mapv(f64::abs).sum();
        assert!(err < 1e-6);
    }

    #[test]
    fn test_ekf_linear_reduces_to_kf_behavior() {
        // Linear system: x⁺ = 0.9 x, z = x + v  (1D)
        let mut ekf = ExtendedKalmanFilter::new(
            array![0.0],
            array![[1.0]],
            array![[0.01]],
            array![[0.1]],
        );

        let f = |x: &Array1<f64>, _: Option<&Array1<f64>>| array![0.9 * x[0]];
        let f_j = |_: &Array1<f64>, _: Option<&Array1<f64>>| array![[0.9]];
        let h = |x: &Array1<f64>| array![x[0]];
        let h_j = |_: &Array1<f64>| array![[1.0]];

        // True trajectory with mild noise in measurements
        let mut true_x = 1.0_f64;
        for _ in 0..20 {
            true_x *= 0.9;
            ekf.predict(&f, &f_j, None);
            ekf.update(&array![true_x + 0.01], &h, &h_j);
        }
        assert!(
            (ekf.state()[0] - true_x).abs() < 0.15,
            "est {} true {true_x}",
            ekf.state()[0]
        );
    }

    #[test]
    fn test_ekf_fd_nonlinear_measurement() {
        // State is angle θ; measure sin(θ)
        let mut ekf = ExtendedKalmanFilter::new(
            array![0.1],
            array![[0.5]],
            array![[1e-4]],
            array![[0.01]],
        );
        let f = |x: &Array1<f64>, _: Option<&Array1<f64>>| array![x[0]]; // static
        let true_theta = 0.5_f64;
        for _ in 0..30 {
            ekf.predict_fd(&f, None, 1e-6);
            ekf.update_fd(&array![true_theta.sin()], |x| array![x[0].sin()], 1e-6);
        }
        assert!(
            (ekf.state()[0] - true_theta).abs() < 0.1,
            "θ est {}",
            ekf.state()[0]
        );
    }
}
