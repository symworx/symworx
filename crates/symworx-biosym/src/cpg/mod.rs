// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Central Pattern Generator (CPG) module.
//! Coupled Van der Pol oscillators for heart, left/right legs, and respiration.
//! Uses RK4 integrator from symworx-math.

use ndarray::Array1;

use symworx_core::math::{VanDerPol, rk4_integrate};

/// Configuration for the coupled CPG model (frequencies, couplings, tau dynamics).
#[derive(Debug, Clone)]
pub struct CpgConfig {
    pub heart_rest: f64,
    pub heart_max_delta: f64,
    pub legs_rest: f64,
    pub legs_max: f64,
    pub resp_rest: f64,
    pub resp_max_delta: f64,
    pub loco_to_cardio: f64,
    pub cardio_to_loco: f64,
    pub left_right: f64,
    pub rsa: f64,
    pub cardio_to_resp: f64,
    pub loco_to_resp: f64,
    pub epsilon: f64, // tau adaptation rate
    pub tau_initial: f64,
}

impl Default for CpgConfig {
    fn default() -> Self {
        Self {
            heart_rest: 1.0,
            heart_max_delta: 0.5,
            legs_rest: 1.0,
            legs_max: 2.0,
            resp_rest: 0.3,
            resp_max_delta: 0.2,
            loco_to_cardio: 0.1,
            cardio_to_loco: 0.05,
            left_right: 0.8,
            rsa: 0.3,
            cardio_to_resp: 0.1,
            loco_to_resp: 0.2,
            epsilon: 0.01,
            tau_initial: 0.0,
        }
    }
}

/// Integrated CPG model with heart, bilateral legs, respiration + dynamic tau.
#[derive(Debug, Clone)]
pub struct SymCpgModel {
    pub heart: VanDerPol,
    pub left: VanDerPol,
    pub right: VanDerPol,
    pub resp: VanDerPol,
    pub tau: f64,
    pub config: CpgConfig,
}

impl SymCpgModel {
    pub fn new(config: Option<CpgConfig>) -> Self {
        let cfg = config.unwrap_or_default();
        Self {
            heart: VanDerPol::new(cfg.heart_rest, 1.0, 0.0),
            left: VanDerPol::new(cfg.legs_rest, 0.3, 0.0),
            right: VanDerPol::new(cfg.legs_rest, -0.3, 0.0),
            resp: VanDerPol::new(cfg.resp_rest, 1.0, 0.0),
            tau: cfg.tau_initial,
            config: cfg,
        }
    }

    /// Compute the full 9D derivative for RK4.
    /// State order: [xh, vh, xl, vl, xr, vr, xresp, vresp, tau]
    pub fn derivatives(&self, t: f64, y: &Array1<f64>) -> Array1<f64> {
        let xh = y[0];
        let vh = y[1];
        let xl = y[2];
        let vl = y[3];
        let xr = y[4];
        let vr = y[5];
        let xresp = y[6];
        let vresp = y[7];
        let tau = y[8];

        // Simple linear tau target for demo (extend with protocol closure later)
        let tau_target = if t < 10.0 { 0.0 } else { 0.8 }; // placeholder ramp

        let omega_h = self.config.heart_rest + self.config.heart_max_delta * tau;
        let omega_l = self.config.legs_rest + self.config.legs_max * tau;
        let omega_r = omega_l;
        let omega_resp = self.config.resp_rest + self.config.resp_max_delta * tau;

        // Forcings (from Python logic)
        let forcing_heart = self.config.loco_to_cardio * (xl + xr) + self.config.rsa * xresp;
        let forcing_left = self.config.cardio_to_loco * xh + self.config.left_right * (xr - xl);
        let forcing_right = self.config.cardio_to_loco * xh + self.config.left_right * (xl - xr);
        let forcing_resp = self.config.cardio_to_resp * xh + self.config.loco_to_resp * (xl + xr);

        let d_h = self.heart.derivative(omega_h, forcing_heart);
        let d_l = self.left.derivative(omega_l, forcing_left);
        let d_r = self.right.derivative(omega_r, forcing_right);
        let d_resp = self.resp.derivative(omega_resp, forcing_resp);

        let d_tau = self.config.epsilon * (tau_target - tau);

        Array1::from(vec![
            d_h.0, d_h.1, d_l.0, d_l.1, d_r.0, d_r.1, d_resp.0, d_resp.1, d_tau,
        ])
    }

    /// Run the coupled CPG using RK4 from symworx-math.
    pub fn run(&self, t_span: (f64, f64), dt: f64) -> (Vec<f64>, Vec<Array1<f64>>) {
        let y0 = Array1::from(vec![
            self.heart.x,
            self.heart.v,
            self.left.x,
            self.left.v,
            self.right.x,
            self.right.v,
            self.resp.x,
            self.resp.v,
            self.tau,
        ]);

        let f = |t: f64, y: &Array1<f64>| self.derivatives(t, y);
        rk4_integrate(f, t_span, y0, dt)
    }
}

/// Simple instantaneous frequency from peaks (basic version).
pub fn instantaneous_freq(x: &Array1<f64>, t: &Array1<f64>, min_dist: usize) -> Array1<f64> {
    // Placeholder: returns zeros. Full peak detection can be added later.
    // For now, this keeps the crate minimal.
    Array1::zeros(x.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    #[test]
    fn test_van_der_pol_derivative() {
        let vdp = VanDerPol::new(1.0, 0.0, 1.0);
        let (dx, dv) = vdp.derivative(1.0, 0.0);
        assert!((dx - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_cpg_run_short() {
        let model = SymCpgModel::new(None);
        let (times, states) = model.run((0.0, 1.0), 0.01);
        assert!(!times.is_empty());
        assert_eq!(states.len(), times.len());
        assert_eq!(states[0].len(), 9); // 9D state
    }
}
