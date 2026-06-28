// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

use ndarray::Array1;

use crate::integrate::rk4_integrate;

/// Van der Pol oscillator.
///
/// A nonlinear oscillator commonly used in biological and
/// physical systems (e.g., heartbeats, neural activity, gait).
#[derive(Debug, Clone)]
pub struct VanDerPol {
    /// The strength of the nonlinear damping.
    /// (higher values produce more relaxed oscillators.
    pub mu: f64,
    /// Current position/displacement.
    pub x: f64,
    /// Current velocity.
    pub v: f64,
}

impl VanDerPol {
    /// Creates a new Van der Pol oscillator with given params/state.
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
