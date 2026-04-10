// filters/adaptive/basic.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use crate::statistics::median;

/// Compute Median Absolute Deviation (MAD)
/// MAD = median(|x - median|) for x in data
///
/// # Arguments
/// * `data` - Input data slice
/// * `med` - Precomputed median of the
///
/// # Returns
/// * MAD value
fn mad(data: &[f64], med: f64) -> f64 {
    let deviations: Vec<f64> = data.iter().map(|x| (x - med).abs()).collect();
    median(&deviations)
}

/// Adaptive median-based outlier replacement filter.
///
/// # Arguments
/// * `data` - Input data slice
/// * `k` - Scaling factor for threshold (e.g., 2.0 for
///
/// # Returns
/// A new `Vec<f64>` where values deviating from the median by more than `theta
pub fn adaptive_median_filter(data: &[f64], k: f64) -> Vec<f64> {
    if data.is_empty() {
        return vec![];
    }

    let med = median(data);
    let mad_val = mad(data, med);
    let theta = k * mad_val;

    data.iter()
        .map(|&x| if (x - med).abs() > theta { med } else { x })
        .collect()
}

/// Adaptive mean-based outlier replacement filter.
/// Threshold = k * standard deviation
///
/// # Arguments
/// * `data` - Input data slice
/// * `k` - Scaling factor for threshold (e.g., 2.0 for
///
/// # Returns
/// A new `Vec<f64>` where values deviating from the mean by more than `theta
pub fn adaptive_mean_filter(data: &[f64], k: f64) -> Vec<f64> {
    if data.is_empty() {
        return vec![];
    }

    let mean = data.iter().sum::<f64>() / (data.len() as f64);
    let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (data.len() as f64);
    let std = variance.sqrt();
    let theta = k * std;

    data.iter()
        .map(|&x| if (x - mean).abs() > theta { mean } else { x })
        .collect()
}

#[cfg(test)]
mod test_median_filter{
    use super::*;
    use crate::statistics::mean;
    
    #[test]
    fn test_adaptive_median_filter() {
        let data = vec![1.0, 1.1, 0.9, 100.0];
        let filtered = adaptive_median_filter(&data, 2.0);

        // median = 1.0, MAD = median(|x - 1|) = 0.1 → theta = 0.2
        // 100.0 is replaced
        assert_eq!(filtered, vec![1.0, 1.1, 0.9, 1.05]);
    }

    #[test]
    fn test_adaptive_mean_filter() {
        let data = vec![10.0, 10.1, 9.9, 50.0];
        let filtered = adaptive_mean_filter(&data, 1.0);
        let mu = mean(&data); // mean = 20.0

        // std small → threshold small → 50 replaced
        assert_eq!(filtered[3], mu); 
    }
}
