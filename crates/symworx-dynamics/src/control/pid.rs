// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Discrete PID controller (textbook / educational form).

/// PID gains and discrete-time options.
#[derive(Debug, Clone)]
pub struct PidConfig {
    /// Proportional gain.
    pub kp: f64,
    /// Integral gain.
    pub ki: f64,
    /// Derivative gain.
    pub kd: f64,
    /// Sample time Δt > 0.
    pub dt: f64,
    /// Optional absolute integral clamp (anti-windup).
    pub integral_limit: Option<f64>,
    /// Optional absolute output clamp.
    pub output_limit: Option<f64>,
}

impl Default for PidConfig {
    fn default() -> Self {
        Self {
            kp: 1.0,
            ki: 0.0,
            kd: 0.0,
            dt: 0.01,
            integral_limit: None,
            output_limit: None,
        }
    }
}

/// Discrete PID: `u = Kp e + Ki ∫e + Kd de/dt`.
#[derive(Debug, Clone)]
pub struct Pid {
    cfg: PidConfig,
    integral: f64,
    prev_error: Option<f64>,
}

impl Pid {
    /// Create a PID controller from configuration.
    pub fn new(cfg: PidConfig) -> Self {
        assert!(cfg.dt > 0.0, "dt must be positive");
        Self {
            cfg,
            integral: 0.0,
            prev_error: None,
        }
    }

    /// Convenience constructor.
    pub fn gains(kp: f64, ki: f64, kd: f64, dt: f64) -> Self {
        Self::new(PidConfig {
            kp,
            ki,
            kd,
            dt,
            ..Default::default()
        })
    }

    /// Reset integrator and derivative memory.
    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.prev_error = None;
    }

    /// Compute control for error `e = setpoint − measurement`.
    pub fn step(&mut self, error: f64) -> f64 {
        self.integral += error * self.cfg.dt;
        if let Some(lim) = self.cfg.integral_limit {
            self.integral = self.integral.clamp(-lim, lim);
        }

        let derivative = match self.prev_error {
            Some(prev) => (error - prev) / self.cfg.dt,
            None => 0.0,
        };
        self.prev_error = Some(error);

        let mut u = self.cfg.kp * error + self.cfg.ki * self.integral + self.cfg.kd * derivative;
        if let Some(lim) = self.cfg.output_limit {
            u = u.clamp(-lim, lim);
        }
        u
    }

    /// Current integrator state (for diagnostics).
    pub fn integral(&self) -> f64 {
        self.integral
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::lti::LtiDiscrete;
    use ndarray::array;

    #[test]
    fn test_p_only_reduces_error() {
        let mut pid = Pid::gains(0.5, 0.0, 0.0, 0.1);
        let u = pid.step(2.0);
        assert!((u - 1.0).abs() < 1e-15);
    }

    #[test]
    fn test_pid_regulates_scalar_plant() {
        // Plant: x⁺ = 0.95 x + 0.1 u, track setpoint 1.0
        let plant = LtiDiscrete::scalar(0.95, 0.1);
        let mut pid = Pid::new(PidConfig {
            kp: 2.0,
            ki: 0.5,
            kd: 0.05,
            dt: 1.0,
            integral_limit: Some(10.0),
            output_limit: Some(20.0),
        });
        let setpoint = 1.0;
        let mut x = array![0.0];
        for _ in 0..80 {
            let e = setpoint - x[0];
            let u = pid.step(e);
            let (x_next, _) = plant.step(&x, Some(&array![u]));
            x = x_next;
        }
        assert!(
            (x[0] - setpoint).abs() < 0.05,
            "final state {} not near setpoint",
            x[0]
        );
    }
}
