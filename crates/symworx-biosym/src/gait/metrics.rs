// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Pure (stateless) gait metric calculations.
//!
//! These helpers contain the core math extracted from `GaitData` methods.
//! They are intentionally decoupled from mutable state so they are easy to
//! test, reuse, and call from generation or analysis code later.

use ndarray::Array1;
use symworx_core::math::series;

/// Aggregated spatiotemporal gait statistics (lightweight analysis result).
/// Mirrors key fields from legacy GaitStatsData for compatibility/porting.
#[derive(Debug, Clone, PartialEq)]
pub struct GaitStats {
    pub n_strides: usize,
    pub mean_stride_time_s: f64,
    pub std_stride_time_s: f64,
    pub cadence_steps_min: Option<f64>,
    pub mean_stride_length_m: Option<f64>,
    pub std_stride_length_m: Option<f64>,
    pub mean_step_length_m: Option<f64>,
    pub mean_vertical_oscillation_m: Option<f64>,
    pub gait_speed_ms: Option<f64>,
    pub symmetry: Option<f64>,
}

fn array_std(a: &Array1<f64>) -> f64 {
    if a.len() < 2 {
        return 0.0;
    }
    // Population std (ddof=0) to match common numpy defaults in legacy
    a.var(0.0).sqrt()
}

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

/// Compute intervals (successive diffs) from event times (strides or steps).
pub fn compute_intervals_from_times(times: &Array1<f64>) -> Array1<f64> {
    let diffs = series::successive_differences(times.as_slice().unwrap_or(&[]));
    Array1::from(diffs)
}

/// Simple relative symmetry index from two paired series (e.g. left/right step intervals or lengths).
/// Returns 0.0 for perfect symmetry (or empty); higher values indicate greater asymmetry.
/// Formula: 2 * |meanL - meanR| / (meanL + meanR)  (bounded [0, ~2] in practice).
pub fn compute_symmetry_index(left: &Array1<f64>, right: &Array1<f64>) -> Option<f64> {
    if left.is_empty() || right.is_empty() {
        return None;
    }
    let ml = left.mean()?;
    let mr = right.mean()?;
    let sum = ml + mr;
    if sum <= 0.0 {
        return Some(0.0);
    }
    Some(2.0 * (ml - mr).abs() / sum)
}

/// Compute aggregate `GaitStats` from available series (all optional inputs after intervals).
/// n_strides is derived from intervals.len() (consistent with "n-1" intervals from k stride times).
pub fn compute_gait_stats(
    stride_intervals: &Array1<f64>,
    stride_lengths: Option<&Array1<f64>>,
    step_lengths: Option<&Array1<f64>>,
    vertical_oscs: Option<&Array1<f64>>,
    provided_speed: Option<f64>,
    symmetry: Option<f64>,
) -> GaitStats {
    let n_strides = stride_intervals.len(); // #intervals ≈ #strides in our convention
    let mean_stride = stride_intervals.mean().unwrap_or(f64::NAN);
    let std_stride = array_std(stride_intervals);
    let cad = compute_cadence(stride_intervals);

    let mean_sl = stride_lengths.and_then(|l| if l.is_empty() { None } else { l.mean() });
    let std_sl = stride_lengths
        .map(|l| array_std(l))
        .filter(|_| stride_lengths.map_or(false, |l| !l.is_empty()));

    let mean_step_l = step_lengths.and_then(|l| if l.is_empty() { None } else { l.mean() });

    let mean_vert = vertical_oscs.and_then(|o| if o.is_empty() { None } else { o.mean() });

    let speed = provided_speed.or_else(|| {
        if let (Some(ml), m) = (mean_sl, mean_stride) {
            if m > 0.0 { Some(ml / m) } else { None }
        } else {
            None
        }
    });

    GaitStats {
        n_strides,
        mean_stride_time_s: mean_stride,
        std_stride_time_s: std_stride,
        cadence_steps_min: cad,
        mean_stride_length_m: mean_sl,
        std_stride_length_m: std_sl,
        mean_step_length_m: mean_step_l,
        mean_vertical_oscillation_m: mean_vert,
        gait_speed_ms: speed,
        symmetry,
    }
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

    #[test]
    fn test_compute_intervals_from_times() {
        let times = array![0.0, 1.1, 2.3];
        let ints = compute_intervals_from_times(&times);
        assert_eq!(ints.len(), 2);
        assert!((ints[0] - 1.1).abs() < 1e-12);
    }

    #[test]
    fn test_compute_symmetry_index() {
        let left = array![1.0, 1.0, 1.0];
        let right = array![1.0, 1.05, 0.95];
        let sym = compute_symmetry_index(&left, &right).unwrap();
        assert!(
            sym >= 0.0 && sym < 0.1,
            "near-symmetric should be low index, got {}",
            sym
        );
        let asym_l = array![1.0, 1.0];
        let asym_r = array![1.2, 1.2];
        let sym_asym = compute_symmetry_index(&asym_l, &asym_r).unwrap();
        assert!((sym_asym - 0.1818).abs() < 0.01); // approx 2*0.2/2.2
    }

    #[test]
    fn test_compute_gait_stats_basic() {
        let ints = array![1.0, 1.0, 1.0];
        let lens = array![1.3, 1.3, 1.3];
        let stats = compute_gait_stats(&ints, Some(&lens), None, None, Some(1.3), Some(0.0));
        assert_eq!(stats.n_strides, 3);
        assert!((stats.mean_stride_time_s - 1.0).abs() < 1e-9);
        assert!((stats.cadence_steps_min.unwrap() - 120.0).abs() < 1e-6);
        assert!((stats.mean_stride_length_m.unwrap() - 1.3).abs() < 1e-9);
        assert_eq!(stats.gait_speed_ms, Some(1.3));
        assert_eq!(stats.symmetry, Some(0.0));
    }
}
