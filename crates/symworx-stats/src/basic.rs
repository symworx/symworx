// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

use std::cmp::Ordering;

/// Calculates the mean of a time series of f64 values.
///
/// # Arguments
/// * `data` - A slice of f64 values for which the mean is to be
///   calculated.
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
    let n = data.len();
    if n == 0 {
        return f64::NAN;
    }

    let mut v = data.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mid = n / 2;
    if n.is_multiple_of(2) {
        (v[mid - 1] + v[mid]) / 2.0
    } else {
        v[mid]
    }
}

/// Compute Median Absolute Deviation (MAD)
/// MAD = median(|x - median|) for x in data
///
/// # Arguments
/// * `data` - Input data slice
/// * `med` - Precomputed median of `data`
///
/// # Returns
/// * MAD value
pub fn mad(data: &[f64], med: f64) -> f64 {
    let deviations: Vec<f64> = data.iter().map(|x| (x - med).abs()).collect();
    median(&deviations)
}

/// Calculate percentiles for a data slice.
/// `p` is a list of percentiles in [0, 100].
///
/// Uses linear interpolation between nearest ranks (NumPy default).
pub fn percentile(data: &[f64], p: Vec<f64>) -> Vec<f64> {
    let n = data.len();
    if n == 0 {
        return vec![f64::NAN; p.len()];
    }

    // Work on a sorted copy
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    p.into_iter()
        .map(|pct| {
            // Clamp percentile to [0, 100]
            let pct = pct.clamp(0.0, 100.0);

            if pct == 0.0 {
                return sorted[0];
            }
            if pct == 100.0 {
                return sorted[n - 1];
            }

            // Convert percentile to fractional index
            let rank = pct / 100.0 * (n - 1) as f64;

            let low = rank.floor() as usize;
            let high = rank.ceil() as usize;

            if low == high {
                sorted[low]
            } else {
                let w = rank - low as f64;
                sorted[low] * (1.0 - w) + sorted[high] * w
            }
        })
        .collect()
}


// TESTS
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
