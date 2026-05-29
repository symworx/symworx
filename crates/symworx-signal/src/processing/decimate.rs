// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Decimation algorithms for visualization and display.
//!
//! These are intended for reducing the number of points shown in plots
//! (e.g. sparklines, line charts) while preserving important visual features
//! such as peaks and troughs. They are **not** meant for anti-aliased
//! resampling for analysis.

/// Decimates a signal using a min-max bucketing strategy.
///
/// This is excellent for visualization because it preserves both the
/// highest peaks and lowest troughs in each bucket, avoiding the
/// flattening effect of averaging or simple striding.
///
/// The output size will be roughly `2 * (data.len() / bucket_size)`.
/// If the input is already shorter than `max_points`, the original data
/// is returned unchanged.
///
/// # Arguments
/// * `data` - The input signal
/// * `max_points` - Desired maximum number of output points (approximate)
///
/// # Example
/// ```ignore
/// let downsampled = min_max_decimate(&long_signal, 800);
/// ```
pub fn min_max_decimate(data: &[f64], max_points: usize) -> Vec<f64> {
    if data.len() <= max_points || max_points < 2 {
        return data.to_vec();
    }

    let mut result = Vec::new();

    // We aim for roughly max_points / 2 buckets, each contributing min + max
    let num_buckets = (max_points / 2).max(1);
    let bucket_size = (data.len() + num_buckets - 1) / num_buckets; // ceil division

    let mut i = 0usize;
    while i < data.len() {
        let end = (i + bucket_size).min(data.len());
        let bucket = &data[i..end];

        if !bucket.is_empty() {
            let mut min_val = f64::INFINITY;
            let mut max_val = f64::NEG_INFINITY;

            for &v in bucket {
                if v < min_val {
                    min_val = v;
                }
                if v > max_val {
                    max_val = v;
                }
            }

            result.push(min_val);
            result.push(max_val);
        }

        i += bucket_size;
    }

    // Trim to desired size if we overshot slightly
    if result.len() > max_points {
        result.truncate(max_points);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_max_decimate_short() {
        let data = vec![1.0, 2.0, 3.0];
        let out = min_max_decimate(&data, 10);
        assert_eq!(out, data);
    }

    #[test]
    fn test_min_max_decimate_preserves_extrema() {
        let data: Vec<f64> = (0..1000).map(|i| (i as f64 * 0.1).sin()).collect();
        let out = min_max_decimate(&data, 40);

        // Should be much shorter
        assert!(out.len() <= 42);
        assert!(out.len() >= 10);

        // Rough sanity: output should still contain near-max and near-min values
        let data_max = data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let data_min = data.iter().fold(f64::INFINITY, |a, &b| a.min(b));

        let out_max = out.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let out_min = out.iter().fold(f64::INFINITY, |a, &b| a.min(b));

        assert!((out_max - data_max).abs() < 0.2);
        assert!((out_min - data_min).abs() < 0.2);
    }
}
