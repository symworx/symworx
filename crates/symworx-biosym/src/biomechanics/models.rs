// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

use ndarray::Array1;

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

/// Simple container for gait event times and derived metrics.
/// Focus on core calculations first.
#[derive(Debug, Clone)]
pub struct GaitData {
    pub fs: f64,
    pub stride_times: Option<Array1<f64>>,
    pub stride_intervals: Option<Array1<f64>>,
    pub left_step_times: Option<Array1<f64>>,
    pub right_step_times: Option<Array1<f64>>,
    pub stride_length: Option<Array1<f64>>,
    pub step_length: Option<Array1<f64>>,
    pub pelvis_vertical_position: Option<Array1<f64>>,
}

impl GaitData {
    /// Initiate a new `Gaitdata` Record
    pub fn new(fs: f64) -> Self {
        Self {
            fs,
            stride_times: None,
            stride_intervals: None,
            left_step_times: None,
            right_step_times: None,
            stride_length: None,
            step_length: None,
            pelvis_vertical_position: None,
        }
    }

    /// Calculate stride intervals from stride times.
    pub fn calculate_stride_intervals(&mut self) -> Option<Array1<f64>> {
        if let Some(ref times) = self.stride_times {
            if times.len() >= 2 {
                let intervals = &times.slice(ndarray::s![1..]) - &times.slice(ndarray::s![..-1]);
                self.stride_intervals = Some(intervals);
                return self.stride_intervals.clone();
            }
        }
        None
    }

    /// Calculate stride length = walking_speed * stride_intervals (in meters).
    pub fn calculate_stride_length(&mut self, walking_speed: Option<f64>) -> Option<Array1<f64>> {
        let speed = walking_speed.unwrap_or(1.3); // sensible default
        if self.stride_intervals.is_none() {
            self.calculate_stride_intervals();
        }
        if let Some(ref intervals) = self.stride_intervals {
            if intervals.is_empty() {
                return None;
            }
            let lengths = intervals.mapv(|dt| speed * dt);
            self.stride_length = Some(lengths.clone());
            Some(lengths)
        } else {
            None
        }
    }

    /// Calculate step length as approximately half of stride length.
    pub fn calculate_step_length(&mut self) -> Option<Array1<f64>> {
        if self.stride_length.is_none() {
            // Try to compute with default speed if not already done
            self.calculate_stride_length(None);
        }
        if let Some(ref stride_len) = self.stride_length {
            let step_len = stride_len.mapv(|l| l / 2.0);
            self.step_length = Some(step_len.clone());
            Some(step_len)
        } else {
            None
        }
    }

    /// Calculate cadence in steps per minute.
    pub fn calculate_cadence(&self) -> Option<f64> {
        self.stride_intervals.as_ref().map(|intervals| {
            if intervals.is_empty() {
                return 0.0;
            }
            let mean_stride = intervals.mean().unwrap_or(0.0);
            if mean_stride > 0.0 {
                120.0 / mean_stride
            } else {
                0.0
            }
        })
    }

    /// Calculate step times (alternating left/right assumption).
    pub fn calculate_step_times(&mut self) {
        if let Some(ref times) = self.stride_times {
            if times.len() >= 2 {
                let left: Vec<f64> = times.iter().step_by(2).copied().collect();
                let right: Vec<f64> = times.iter().skip(1).step_by(2).copied().collect();
                self.left_step_times = Some(Array1::from(left));
                self.right_step_times = Some(Array1::from(right));
            }
        }
    }

    /// Basic vertical oscillation per stride (max - min in each cycle).
    pub fn calculate_vertical_oscillation(&self) -> Option<Array1<f64>> {
        let pos = self.pelvis_vertical_position.as_ref()?;
        let times = self.stride_times.as_ref()?;
        if times.len() < 2 || pos.is_empty() {
            return None;
        }

        let mut oscillations = Vec::new();
        for i in 0..times.len() - 1 {
            let start_t = times[i];
            let end_t = times[i + 1];
            let start_idx = (start_t * self.fs).round() as usize;
            let end_idx = (end_t * self.fs).round() as usize;

            let end_idx = end_idx.min(pos.len());
            if start_idx < end_idx {
                let cycle = pos.slice(ndarray::s![start_idx..end_idx]);
                if let Some(&max_v) = cycle.iter().max_by(|a, b| a.partial_cmp(b).unwrap()) {
                    if let Some(&min_v) = cycle.iter().min_by(|a, b| a.partial_cmp(b).unwrap()) {
                        oscillations.push(max_v - min_v);
                    }
                }
            }
        }
        if oscillations.is_empty() {
            None
        } else {
            Some(Array1::from(oscillations))
        }
    }
}


// TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    #[test]
    fn test_gait_params_defaults() {
        let p = GaitParams::default().with_defaults();
        assert!(p.leg_length.is_some());
        assert!(p.cadence.is_some());
        assert!(p.stride_time() > 0.0);
    }

    #[test]
    fn test_calculate_stride_intervals() {
        let mut data = GaitData::new(100.0);
        data.stride_times = Some(array![0.0, 1.2, 2.4, 3.6]);
        let intervals = data.calculate_stride_intervals().unwrap();
        assert_eq!(intervals.len(), 3);
        assert!((intervals[0] - 1.2).abs() < 1e-9);
    }

    #[test]
    fn test_calculate_stride_length() {
        let mut data = GaitData::new(100.0);
        data.stride_times = Some(array![0.0, 1.0, 2.0]);
        data.calculate_stride_intervals();
        let lengths = data.calculate_stride_length(Some(1.3)).unwrap();
        assert_eq!(lengths.len(), 2);
        assert!((lengths[0] - 1.3).abs() < 1e-9);
    }

    #[test]
    fn test_calculate_step_length() {
        let mut data = GaitData::new(100.0);
        data.stride_times = Some(array![0.0, 1.0, 2.0]);
        data.calculate_stride_intervals();
        data.calculate_stride_length(Some(1.3));
        let steps = data.calculate_step_length().unwrap();
        assert!((steps[0] - 0.65).abs() < 1e-9);
    }

    #[test]
    fn test_calculate_cadence() {
        let mut data = GaitData::new(100.0);
        data.stride_times = Some(array![0.0, 1.0, 2.0]);
        data.calculate_stride_intervals();
        let cad = data.calculate_cadence().unwrap();
        assert!((cad - 120.0).abs() < 1e-6);
    }

    #[test]
    fn test_step_times() {
        let mut data = GaitData::new(100.0);
        data.stride_times = Some(array![0.0, 1.0, 2.0, 3.0]);
        data.calculate_step_times();
        assert!(data.left_step_times.is_some());
        assert!(data.right_step_times.is_some());
    }
}
