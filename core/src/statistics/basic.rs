// statistics/basic.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use std::cmp::Ordering;

/// Calculates the mean of a time series of f64 values.
///
/// # Arguments
/// * `data` - A slice of f64 values for which the mean is to be
///            calculated.
///
/// # Returns
/// The mean value as an f64. If the input slice is empty, it returns NaN.
pub fn mean(data: &[f64]) -> f64 {
    if data.is_empty() {
        return f64::NAN;
    }
    data.iter().sum::<f64>() / (data.len() as f64)
}


/// Calculates the median of a slice of f64 values.
/// 
/// # Arguments
/// * `data` - A slice of f64 values for which the median is to be calculated.
/// 
/// # Returns
/// The median value as an f64. If the input slice is empty, it returns NaN.
pub fn median(data: &[f64]) -> f64 {
    if data.is_empty() {
        return f64::NAN;
    }

    let length = data.len();
    let mut sorted_data = data.to_vec();

    sorted_data.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

    if length % 2 == 0 {
        let mid = length / 2;
        (sorted_data[mid - 1] + sorted_data[mid]) / 2.0
    } else {
        sorted_data[length / 2]
    }
}

// ==========================================================
// TESTS
// ==========================================================
#[cfg(test)]
mod test_mean {
    use super::*;

    #[test]
    fn test_mean_odd() {
        let data = vec![3.0, 4.0, 5.0,];
        let mu = mean(&data);
        assert_eq!(mu, 4.0_f64);
    }

    #[test]
    fn test_mean_empty() {
        let data: Vec<f64> = vec![];
        let mu = mean(&data);
        assert!(mu.is_nan());
    }
}

#[cfg(test)]
mod test_median {
    use super::*;

    #[test]
    fn test_median_odd() {
        let data = vec![3.0, 1.0, 4.0, 1.5, 5.0];
        let med = median(&data);
        assert_eq!(med, 3.0_f64);
    }

    #[test]
    fn test_median_even() {
        let data = vec![1.0, 3.0, 2.0, 4.0];
        let med = median(&data);
        assert_eq!(med, 2.5_f64);
    }

    #[test]
    fn test_median_empty() {
        let data: Vec<f64> = vec![];
        let med = median(&data);
        assert!(med.is_nan());
    }
}
