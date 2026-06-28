// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Core gait simulation / analysis parameters.
//! Adapted from Python GaitSimulationParams with idiomatic Rust.

/// Core gait simulation / analysis parameters.
/// Adapted from Python GaitSimulationParams with idiomatic Rust.
#[derive(Debug, Clone)]
pub struct GaitParams {
    /// Walking speed (m/s)
    pub walking_speed: f64,
    /// Step length (meters)
    pub step_length: f64,
    /// Cadence (steps/min)
    pub cadence: Option<f64>,
    /// Mass (kg)
    pub mass: f64,
    /// Height (meters)
    pub height: f64,
    /// Leg Length (meters)
    pub leg_length: Option<f64>,
    /// Stride variability (CV)
    pub stride_variability: f64,
    /// Asymmetry [0..1]
    pub asymmetry: f64,
}

impl Default for GaitParams {
    fn default() -> Self {
        Self {
            walking_speed: 1.3,
            step_length: 0.65,
            cadence: None,
            mass: 70.0,
            height: 1.75,
            leg_length: None,
            stride_variability: 0.03,
            asymmetry: 0.0,
        }
    }
}

impl GaitParams {
    pub fn new() -> Self {
        Self::default()
    }

    /// Estimate leg length (~53% of height) and cadence if not provided.
    pub fn with_defaults(mut self) -> Self {
        if self.leg_length.is_none() {
            self.leg_length = Some(self.height * 0.53);
        }
        if self.cadence.is_none() {
            self.cadence = Some((self.walking_speed / self.step_length) * 60.0);
        }
        self
    }

    pub fn stride_time(&self) -> f64 {
        if let Some(cad) = self.cadence {
            120.0 / cad
        } else {
            self.step_length / self.walking_speed * 2.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gait_params_with_defaults() {
        let params = GaitParams::default().with_defaults();
        assert!(params.leg_length.is_some());
        let leg_len = params.leg_length.unwrap();
        assert!((leg_len - 1.75 * 0.53).abs() < 0.01);
        assert!(params.cadence.is_some());
    }
}
