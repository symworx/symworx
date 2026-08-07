// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

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

/// Calculates the population standard deviation of a slice of f64 values.
///
/// sd = sqrt( mean( (x_i - mean)^2 ) )  (divide by n)
///
/// Returns NaN for n < 2 (consistent with other basic stats on insufficient data).
pub fn std_dev(data: &[f64]) -> f64 {
    let n = data.len();
    if n < 2 {
        return f64::NAN;
    }
    let mu = mean(data);
    if mu.is_nan() {
        return f64::NAN;
    }
    let variance: f64 = data.iter().map(|&x| (x - mu).powi(2)).sum::<f64>() / n as f64;
    variance.sqrt()
}

/// Calculates the sample standard deviation (Bessel's correction, divide by n-1).
///
/// Returns NaN for n < 2.
pub fn std_dev_sample(data: &[f64]) -> f64 {
    let n = data.len();
    if n < 2 {
        return f64::NAN;
    }
    let mu = mean(data);
    if mu.is_nan() {
        return f64::NAN;
    }
    let variance: f64 = data.iter().map(|&x| (x - mu).powi(2)).sum::<f64>() / (n - 1) as f64;
    variance.sqrt()
}

/// Coefficient of variation (CV = std_dev / |mean|), using population std.
///
/// Returns NaN if mean is zero or data insufficient.
pub fn cv(data: &[f64]) -> f64 {
    let mu = mean(data);
    if mu == 0.0 || mu.is_nan() {
        return f64::NAN;
    }
    let sd = std_dev(data);
    if sd.is_nan() {
        return f64::NAN;
    }
    sd / mu.abs()
}

// TESTS
#[cfg(test)]
mod test_mean {
    use super::*;

    #[test]
    fn test_mean_odd() {
        let data = vec![3.0, 4.0, 5.0];
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

#[cfg(test)]
mod test_std_dev {
    use super::*;

    #[test]
    fn test_std_dev_basic() {
        let data = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        // Known population sd ≈ 2.0 (for this set)
        let sd = std_dev(&data);
        assert!((sd - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_std_dev_sample() {
        let data = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let sd = std_dev_sample(&data);
        // sqrt( variance with n-1 ) > population
        assert!(sd > 2.0 && sd < 2.2);
    }

    #[test]
    fn test_std_dev_insufficient() {
        assert!(std_dev(&[42.0]).is_nan());
        assert!(std_dev(&[]).is_nan());
    }

    #[test]
    fn test_cv() {
        let data = vec![10.0, 20.0, 30.0];
        let c = cv(&data);
        assert!((c - 0.40824829046).abs() < 1e-8); // sd=8.16496... / mean=20
    }
}
