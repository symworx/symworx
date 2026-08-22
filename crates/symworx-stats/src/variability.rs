// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Time series variability and successive difference metrics
//!
//! General-purpose functions for analyzing variability in any sequential
//! signal (e.g. heart rate intervals, gait stride times, respiration cycles, etc.).
//!
//! The core `successive_differences` primitive is signed and lives in
//! `symworx-math`. This module re-exports it and builds higher-level
//! variability descriptors on top.

/// Returns the absolute differences between consecutive elements.
///
/// Re-exported from `symworx-math`.
pub use series::successive_absolute_differences;
/// Returns the signed differences between consecutive elements.
///
/// This is the canonical "successive differences" primitive, re-exported
/// from `symworx-math` for convenience.
///
/// It preserves direction (e.g. deceleration vs acceleration in intervals).
///
/// For the absolute version, see [`successive_absolute_differences`].
pub use series::successive_differences;
use symworx_math::series;

/// Mean of successive absolute differences (MSD).
///
/// Uses absolute diffs (conventional for HRV / gait variability). Empty → `NaN`.
pub fn mean_successive_differences(data: &[f64]) -> f64 {
    let diffs = successive_absolute_differences(data);
    if diffs.is_empty() {
        return f64::NAN;
    }
    diffs.iter().sum::<f64>() / diffs.len() as f64
}

/// Root mean square of successive differences (RMSSD).
///
/// Short-term variability. `len < 3` → `NaN`.
pub fn rmssd(data: &[f64]) -> f64 {
    if data.len() < 3 {
        return f64::NAN;
    }
    let sum_sq: f64 = data.windows(2).map(|w| (w[1] - w[0]).abs().powi(2)).sum();

    (sum_sq / (data.len() - 1) as f64).sqrt()
}

/// Standard deviation of successive differences (SDSD). `len < 2` → `NaN`.
pub fn sd_successive_differences(data: &[f64]) -> f64 {
    if data.len() < 2 {
        return f64::NAN;
    }
    let diffs = successive_differences(data);
    let mean = diffs.iter().sum::<f64>() / diffs.len() as f64;

    diffs.iter().map(|&d| (d - mean).powi(2)).sum::<f64>().sqrt() / (diffs.len() as f64).sqrt()
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_successive_differences() {
        let data = [1.0, 2.0, 4.0];
        // Now signed (not absolute)
        assert_eq!(successive_differences(&data), vec![1.0, 2.0]);
    }

    #[test]
    fn test_successive_differences_signed() {
        let data = [4.0, 2.0, 5.0];
        assert_eq!(successive_differences(&data), vec![-2.0, 3.0]);
    }

    #[test]
    fn test_mean_successive_difference() {
        let data = [1.0, 2.0, 4.0];
        // Uses absolute differences internally
        assert_eq!(mean_successive_differences(&data), 1.5);
    }

    #[test]
    fn test_rmssd() {
        let data = [1.0, 2.0, 4.0];
        // diffs = [1,2], rms = sqrt( (1+4)/2 ) = sqrt(2.5)
        assert!((rmssd(&data) - 2.5f64.sqrt()).abs() < 1e-8);
    }

    #[test]
    fn test_sd_successive_difference() {
        let data = [1.0, 2.0, 4.0];
        // diffs=[1,2], pop std (ddof=0) = 0.5
        assert!((sd_successive_differences(&data) - 0.5).abs() < 1e-8);
    }

    #[test]
    fn test_edge_cases() {
        assert!(mean_successive_differences(&[]).is_nan());
        assert!(rmssd(&[1.0, 2.0]).is_nan());
        assert!(sd_successive_differences(&[5.0]).is_nan());
    }
}
