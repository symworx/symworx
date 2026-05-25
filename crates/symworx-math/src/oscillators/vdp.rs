// Copyright (C) 2026 cSYMd, All rights reserved.

use ndarray::Array1;
use crate::integrate::rk4_integrate;

/// Van der Pol oscillator parameters and state.
#[derive(Debug, Clone)]
pub struct VanDerPol {
    pub mu: f64,
    pub x: f64,
    pub v: f64,
}

impl VanDerPol {
    pub fn new(mu: f64, x: f64, v: f64) -> Self {
        Self { mu, x, v }
    }

    /// Compute derivatives: dx/dt = v, dv/dt = mu*(1 - x^2)*v - omega^2 * x + forcing
    pub fn derivative(&self, omega: f64, forcing: f64) -> (f64, f64) {
        let dx = self.v;
        let dv = self.mu * (1.0 - self.x * self.x) * self.v - omega * omega * self.x + forcing;
        (dx, dv)
    }
}
