// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Outlier detection and replacement filters.
//!
//! Adaptive filtering methods for removing/replacing outliers in signals.

use symworx_stats::{
    mad,
    mean,
    median,
};

/// Adaptive mean-based outlier replacement.
///
/// Replaces values that deviate from the mean by more than `k * std`
/// with the mean value.
pub fn adaptive_mean_filter(data: &[f64], k: f64) -> Vec<f64> {
    if data.is_empty() {
        return vec![];
    }

    let mu = mean(data);
    let variance = data.iter().map(|&x| (x - mu).powi(2)).sum::<f64>() / data.len() as f64;
    let std = variance.sqrt();
    let threshold = k * std;

    data.iter()
        .map(|&x| if (x - mu).abs() > threshold { mu } else { x })
        .collect()
}

/// Adaptive median-based outlier replacement using MAD.
///
/// Replaces values that deviate from the median by more than `k * MAD`
/// with the median value.
///
/// This method is more robust to outliers than the mean-based version.
pub fn adaptive_median_filter(data: &[f64], k: f64) -> Vec<f64> {
    if data.is_empty() {
        return vec![];
    }

    let med = median(data);
    let mad_val = mad(data, med);
    let threshold = k * mad_val;

    data.iter()
        .map(|&x| if (x - med).abs() > threshold { med } else { x })
        .collect()
}

// ——— Tests ———

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_mean_filter() {
        let data = vec![10.0, 10.1, 9.9, 50.0];
        // Use k=1.5 so the (contaminated) z-score still flags the outlier for this small data.
        // (With mean/std the max z for a single outlier in 4 pts is ~1.73.)
        let filtered = adaptive_mean_filter(&data, 1.5);

        assert_eq!(filtered.len(), 4);
        assert!((filtered[3] - 20.0).abs() < 1e-6); // replaced with (contaminated) mean
    }

    #[test]
    fn test_adaptive_median_filter() {
        let data = vec![1.0, 1.1, 0.9, 100.0];
        let filtered = adaptive_median_filter(&data, 2.0);

        assert_eq!(filtered.len(), 4);
        // Median of 4 pts (even) is avg of 2 middle after sort: (1.0 + 1.1)/2 = 1.05
        // MAD small → 100.0 replaced with median (1.05)
        assert_eq!(filtered[0], 1.0);
        assert_eq!(filtered[1], 1.1);
        assert_eq!(filtered[2], 0.9);
        assert_eq!(filtered[3], 1.05); // replaced
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(adaptive_mean_filter(&[], 2.0), vec![]);
        assert_eq!(adaptive_median_filter(&[], 2.0), vec![]);
    }
}
