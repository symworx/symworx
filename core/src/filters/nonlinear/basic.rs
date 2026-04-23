#![allow(unused_imports)]
#![allow(dead_code)]

// filters/adaptive/basic.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use crate::statistics::median;

/// Median-based outlier replacement filter.
///
/// # Arguments
/// * `data`  - Slice of f64 values
/// * `theta` - Threshold for replacement (absolute deviation from median)
///
/// # Returns
/// A new Vec<f64> where values deviating from the median by more than `theta`
/// are replaced with the median.
pub fn median_filter(data: &[f64], theta: f64) -> Vec<f64> {
    if data.is_empty() {
        return vec![];
    }

    let p50 = median(data);

    data.iter()
        .map(|&x| {
            if (x - p50).abs() > theta {
                p50
            } else {
                x
            }
        })
        .collect()
}

/// Mean-based outlier replacement filter.
///
/// # Arguments
/// * `data`  - Slice of f64 values
/// * `theta` - Threshold for replacement (absolute deviation from mean)
///
/// # Returns
/// A new Vec<f64> where values deviating from the mean by more than `theta
/// are replaced with the mean.
pub fn mean_filter(data: &[f64], theta: f64) -> Vec<f64> {
    if data.is_empty() {
        return vec![];
    }

    let mean = data.iter().sum::<f64>() / (data.len() as f64);

    data.iter()
        .map(|&x| {
            if (x - mean).abs() > theta {
                mean
            } else {
                x
            }
        })
        .collect()
}

// === UNIT TESTS ===========================================
// --- Unit tests for median_filter -------------------------
#[cfg(test)]
mod test_median_filter {
    use super::*;

    #[test]
    fn test_small_vector() {
        let data = vec![1.0, 1.1, 0.9];
        let filtered = median_filter(&data, 0.05);

        // median is 1.0
        assert_eq!(filtered, vec![1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_no_replacement() {
        let data = vec![10.0, 10.1, 9.9];
        let filtered = median_filter(&data, 1.0);

        assert_eq!(filtered, data);
    }

    #[test]
    fn test_with_replacement() {
        let data = vec![1.0, 100.0, 1.2, 0.9];
        let filtered = median_filter(&data, 0.5);

        // median is 1.1
        assert_eq!(filtered, vec![1.0, 1.1, 1.2, 0.9]);
    }
}

// --- Unit tests for mean_filter -------------------------
#[cfg(test)]
mod test_mean_filter {
    use super::*;

    #[test]
    fn test_small_vector() {
        let data = vec![1.0, 1.1, 0.9];
        let filtered = mean_filter(&data, 0.05);

        assert_eq!(filtered, vec![1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_no_replacement() {
        let data = vec![10.0, 10.1, 9.9];
        let filtered = mean_filter(&data, 1.0);

        assert_eq!(filtered, data);
    }
}
