// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

use ndarray::Array1;

use super::metrics;

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
    /// Initiate a new `GaitData` Record
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
                let intervals = metrics::compute_stride_intervals(times);
                self.stride_intervals = Some(intervals.clone());
                return Some(intervals);
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
            let lengths = metrics::compute_stride_lengths(intervals, speed);
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
        self.stride_intervals
            .as_ref()
            .and_then(metrics::compute_cadence)
    }

    /// Calculate step times (alternating left/right assumption).
    pub fn calculate_step_times(&mut self) {
        if let Some(ref times) = self.stride_times {
            if times.len() >= 2 {
                let (left, right) = metrics::split_step_times(times);
                self.left_step_times = Some(left);
                self.right_step_times = Some(right);
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
    use ndarray::array;

    use super::*;

    #[test]
    fn test_gait_params_defaults() {
        // Note: this test lives here for historical reasons but exercises GaitParams.
        // In a future pass it can move to params.rs tests.
        let p = crate::GaitParams::default().with_defaults();
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
