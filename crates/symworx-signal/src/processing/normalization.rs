// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

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
    fn test_zscore() {
        let data = vec![1.0, 2.0, 3.0];
        let zscored = zscore(&data);
        let mean = zscored.iter().sum::<f64>() / zscored.len() as f64;
        let std_dev =
            (zscored.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / zscored.len() as f64).sqrt();

        assert!((mean - 0.0).abs() < 1e-6);
        assert!((std_dev - 1.0).abs() < 1e-5);
    }
}
