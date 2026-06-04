// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Pure (stateless) gait metric calculations.
//!
//! These helpers contain the core math extracted from `GaitData` methods.
//! They are intentionally decoupled from mutable state so they are easy to
//! test, reuse, and call from generation or analysis code later.

use ndarray::Array1;
use symworx_core::math::series;

/// Compute successive (signed) differences between stride times.
///
/// This is sourced from the canonical implementation in `symworx-math`
/// (re-exported through `symworx-core`).
pub fn compute_stride_intervals(stride_times: &Array1<f64>) -> Array1<f64> {
    let diffs = series::successive_differences(stride_times.as_slice().unwrap_or(&[]));
    Array1::from(diffs)
}

/// Scale stride intervals by walking speed to obtain lengths (meters).
pub fn compute_stride_lengths(intervals: &Array1<f64>, walking_speed: f64) -> Array1<f64> {
    intervals.mapv(|dt| walking_speed * dt)
}

/// Compute cadence (steps/min) from mean stride interval.
/// Returns `None` for empty or zero-length data.
pub fn compute_cadence(intervals: &Array1<f64>) -> Option<f64> {
    if intervals.is_empty() {
        return None;
    }
    let mean_stride = intervals.mean()?;
    if mean_stride > 0.0 {
        Some(120.0 / mean_stride)
    } else {
        None
    }
}

/// Split stride times into alternating left / right step times (simple even/odd).
pub fn split_step_times(stride_times: &Array1<f64>) -> (Array1<f64>, Array1<f64>) {
    let left: Vec<f64> = stride_times.iter().step_by(2).copied().collect();
    let right: Vec<f64> = stride_times.iter().skip(1).step_by(2).copied().collect();
    (Array1::from(left), Array1::from(right))
}

// --- Focused unit tests for the pure helpers ---

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    #[test]
    fn test_compute_stride_intervals_basic() {
        let times = array![0.0, 1.2, 2.4, 3.6];
        let ints = compute_stride_intervals(&times);
        assert_eq!(ints.len(), 3);
        assert!((ints[0] - 1.2).abs() < 1e-12);
        assert!((ints[2] - 1.2).abs() < 1e-12);
    }

    #[test]
    fn test_compute_stride_intervals_too_short() {
        let times = array![0.0];
        let ints = compute_stride_intervals(&times);
        assert_eq!(ints.len(), 0);
    }

    #[test]
    fn test_compute_stride_lengths() {
        let intervals = array![1.0, 1.1];
        let lengths = compute_stride_lengths(&intervals, 1.3);
        assert!((lengths[0] - 1.3).abs() < 1e-12);
        assert!((lengths[1] - 1.43).abs() < 1e-12);
    }

    #[test]
    fn test_compute_cadence() {
        let intervals = array![1.0, 1.0, 1.0];
        let cad = compute_cadence(&intervals).unwrap();
        assert!((cad - 120.0).abs() < 1e-9);
    }

    #[test]
    fn test_split_step_times() {
        let times = array![0.0, 1.0, 2.0, 3.0, 4.0];
        let (left, right) = split_step_times(&times);
        assert_eq!(left.len(), 3);
        assert_eq!(right.len(), 2);
        assert!((left[1] - 2.0).abs() < 1e-12);
    }
}
