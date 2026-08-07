// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

/// Min-Max normalization scales the data to a fixed range, typically [0, 1]
///
/// # Arguments
/// * `data` - A vector of f64 values to be normalized
///
/// # Returns
/// A vector of f64 values normalized to the range [0, 1]
pub fn normalize(data: &[f64]) -> Vec<f64> {
    let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    data.iter().map(|x| (x - min) / (max - min)).collect()
}

/// Scale/normalize the time series data to a percentage scale [0, 100].
/// The minimum value maps to 0 and the maximum (length) always maps to 100.
///
/// # Arguments
/// * `data` - A vector of f64 values to be normalized
///
/// # Returns
/// A vector of f64 values normalized to percent.
/// These data can be re-interpolated using the `Resample` method.
/// TODO: add docs on resample
pub fn scale_to_percent(data: &[f64]) -> Vec<f64> {
    if data.is_empty() {
        return Vec::new();
    }

    let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    if max == min {
        // all values are the same
        vec![0.0; data.len()]
    } else {
        data.iter().map(|x| 100.0 * (x - min) / (max - min)).collect()
    }
}

/// Z-score normalization standardizes the data to have a mean of 0 and a standard deviation of 1
///
/// # Arguments
/// * `data` - A vector of f64 values to be normalized
///
/// # Returns
/// A vector of f64 values normalized to have a mean of 0 and a standard deviation of 1
pub fn zscore(data: &[f64]) -> Vec<f64> {
    let mean = data.iter().sum::<f64>() / data.len() as f64;
    let std_dev = (data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / data.len() as f64).sqrt();
    data.iter().map(|x| (x - mean) / std_dev).collect()
}

// TESTS
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        let data = vec![1.0, 2.0, 3.0];
        let normalized = normalize(&data);
        assert_eq!(normalized, vec![0.0, 0.5, 1.0]);
    }

    #[test]
    fn test_scale_to_percent() {
        // Test standard scaling
        let data = vec![1.0, 2.0, 3.0];
        let percents = scale_to_percent(&data);

        assert!((percents[0] - 0.0).abs() < 1e-9);
        assert!((percents[1] - 50.0).abs() < 1e-9);
        assert!((percents[2] - 100.0).abs() < 1e-9);

        // Test constant data
        let data_const = vec![5.0, 5.0, 5.0];
        let percents_const = scale_to_percent(&data_const);
        assert_eq!(percents_const, vec![0.0, 0.0, 0.0]);

        // Test empty data
        let data_empty = vec![];
        let percents_empty = scale_to_percent(&data_empty);
        assert!(percents_empty.is_empty());
    }

    #[test]
    fn test_zscore() {
        let data = vec![1.0, 2.0, 3.0];
        let zscored = zscore(&data);
        let mean = zscored.iter().sum::<f64>() / zscored.len() as f64;
        let std_dev = (zscored.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / zscored.len() as f64).sqrt();

        assert!((mean - 0.0).abs() < 1e-6);
        assert!((std_dev - 1.0).abs() < 1e-5);
    }
}
