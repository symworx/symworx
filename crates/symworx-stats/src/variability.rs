// Copyright (C) 2026 cSYMd, All rights reserved.

//! Time series variability and successive difference metrics
//!
//! General-purpose functions for analyzing variability in any sequential
//! signal (e.g. heart rate intervals, gait stride times, respiration cycles, etc.).

/// Returns the absolute differences between consecutive elements.
///
/// # Arguments
/// * `data` - Input signal
///
/// # Returns
/// Vector of successive differences. Returns empty vector if `data.len() < 2`.
pub fn successive_differences(data: &[f64]) -> Vec<f64> {
    if data.len() < 2 {
        return Vec::new();
    }
    data.windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .collect()
}

/// Computes the mean of successive absolute differences.
///
/// # Arguments
/// * `data` - Input signal
///
/// # Returns
/// Mean successive difference. Returns `NaN` if `data` is empty.
pub fn mean_successive_difference(data: &[f64]) -> f64 {
    let diffs = successive_differences(data);
    if diffs.is_empty() {
        return f64::NAN;
    }
    diffs.iter().sum::<f64>() / diffs.len() as f64
}

/// Root Mean Square of Successive Differences (RMSSD)
///
/// A common measure of short-term variability.
///
/// # Arguments
/// * `data` - Input signal
///
/// # Returns
/// RMSSD value. Returns `NaN` if `data.len() < 3`.
pub fn rmssd(data: &[f64]) -> f64 {
    if data.len() < 3 {
        return f64::NAN;
    }
    let sum_sq: f64 = data
        .windows(2)
        .map(|w| (w[1] - w[0]).abs().powi(2))
        .sum();

    (sum_sq / (data.len() - 1) as f64).sqrt()
}

/// Standard Deviation of Successive Differences (often called SDSD)
///
/// # Arguments
/// * `data` - Input signal
///
/// # Returns
/// Standard deviation of successive differences. Returns `NaN` if `data.len() < 2`.
pub fn sd_successive_difference(data: &[f64]) -> f64 {
    if data.len() < 2 {
        return f64::NAN;
    }
    let diffs = successive_differences(data);
    let mean = diffs.iter().sum::<f64>() / diffs.len() as f64;

    diffs.iter()
        .map(|&d| (d - mean).powi(2))
        .sum::<f64>()
        .sqrt() / (diffs.len() as f64).sqrt()
}


// --- TESTS ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_successive_differences() {
        let data = [1.0, 2.0, 4.0];
        assert_eq!(successive_differences(&data), vec![1.0, 2.0]);
    }

    #[test]
    fn test_mean_successive_difference() {
        let data = [1.0, 2.0, 4.0];
        assert_eq!(mean_successive_difference(&data), 1.5);
    }

    #[test]
    fn test_rmssd() {
        let data = [1.0, 2.0, 4.0];
        assert!((rmssd(&data) - 1.41421356237).abs() < 1e-8); // sqrt(2)
    }

    #[test]
    fn test_sd_successive_difference() {
        let data = [1.0, 2.0, 4.0];
        assert!((sd_successive_difference(&data) - 0.70710678118).abs() < 1e-8);
    }

    #[test]
    fn test_edge_cases() {
        assert!(mean_successive_difference(&[]).is_nan());
        assert!(rmssd(&[1.0, 2.0]).is_nan());
        assert!(sd_successive_difference(&[5.0]).is_nan());
    }
}
