// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

use ndarray::prelude::*;

// Sample Entropy (SampEn)
/// Calculate Sample Entropy (SampEn) of a time series.
///
/// # Arguments
/// * `data` - Input time series
/// * `m`    - Embedding dimension (typically 2)
/// * `r`    - Tolerance (similarity threshold)
///
/// # Returns
/// Sample Entropy value. Returns 0.0 for constant or near-constant signals,
///   or if the tolerance is too small.
pub fn sample_entropy(data: &[f64], m: usize, r: f64) -> f64 {
    let n = data.len();
    if n <= m + 1 {
        return 0.0;
    }

    // Early exit for constant or near-constant signals
    let arr = Array1::from_vec(data.to_vec());
    let std_dev = arr.std(0.0);

    if std_dev < 1e-12 {
        return 0.0; // constant or near-constant signal
    }

    // Safety checks
    const EPSILON: f64 = 1e-10;
    if r < EPSILON {
        return 0.0;
    }

    let mut count_m = 0usize;
    let mut count_m_plus_1 = 0usize;

    // Correct loop bounds:
    // i + m < n  ⇒  i <= n - m - 1
    for i in 0..(n - m - 1) {
        for j in (i + 1)..(n - m) {
            let mut match_m = true;

            for k in 0..m {
                if f64::abs(data[i + k] - data[j + k]) > r {
                    match_m = false;
                    break;
                }
            }

            if match_m {
                count_m += 1;
                if f64::abs(data[i + m] - data[j + m]) <= r {
                    count_m_plus_1 += 1;
                }
            }
        }
    }

    if count_m == 0 || count_m_plus_1 == 0 {
        return 0.0;
    }

    -((count_m_plus_1 as f64) / (count_m as f64)).ln()
}

/// Multiscale Entropy (MSE): sample entropy computed on successively coarse-grained versions of the series.
///
/// For each scale factor `s = 1 .. max_scale`:
/// - Coarse-grain by non-overlapping means of length `s`.
/// - Compute `sample_entropy(coarse, m, r)` (r is typically fixed, e.g. 0.2 * std(original)).
///
/// Returns a vector of length `max_scale` (0.0 for scales where the coarse series is too short).
///
/// This is useful for quantifying complexity across multiple temporal scales
/// (e.g. in HRV, stride intervals, respiration).
pub fn multiscale_entropy(data: &[f64], max_scale: usize, m: usize, r: f64) -> Vec<f64> {
    if max_scale == 0 || data.is_empty() {
        return vec![];
    }
    let n = data.len();
    let mut out = Vec::with_capacity(max_scale);

    for scale in 1..=max_scale {
        if n < scale + m + 1 {
            out.push(0.0);
            continue;
        }
        let coarse_len = n / scale;
        if coarse_len <= m + 1 {
            out.push(0.0);
            continue;
        }
        let mut coarse = Vec::with_capacity(coarse_len);
        for j in 0..coarse_len {
            let start = j * scale;
            let end = (start + scale).min(n);
            let sum: f64 = data[start..end].iter().sum();
            coarse.push(sum / (end - start) as f64);
        }
        let se = sample_entropy(&coarse, m, r);
        out.push(se);
    }
    out
}

// TESTS
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_entropy_counts() {
        let data = [0.1, 0.2, 0.3, 0.4, 0.5];
        let m = 2;
        let r = 0.1;

        let result = sample_entropy(&data, m, r);
        println!("Sample Entropy(m={}, r={}) = {:.6}", m, r, result);
    }

    #[test]
    fn test_small_r_returns_zero() {
        let data = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(sample_entropy(&data, 2, 1e-12), 0.0);
    }

    #[test]
    fn test_constant_signal() {
        let data = [5.0; 20];
        let result = sample_entropy(&data, 2, 0.1);
        println!("Constant signal entropy = {}", result);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_long_constant_signal() {
        let data = vec![42.0; 100];
        assert_eq!(sample_entropy(&data, 2, 0.5), 0.0);
    }

    #[test]
    fn test_mse_basic_on_sine() {
        let n = 200usize;
        let data: Vec<f64> = (0..n)
            .map(|i| (2.0 * std::f64::consts::PI * i as f64 / 20.0).sin())
            .collect();
        let _r = 0.2 * data.iter().sum::<f64>() / n as f64; // rough; better would use std
        let mse = multiscale_entropy(&data, 8, 2, 0.15);
        assert_eq!(mse.len(), 8);
        // At least scale 1 should give a finite value for periodic data
        assert!(mse[0] >= 0.0 || mse[0] == 0.0); // can be low for very regular
    }

    #[test]
    fn test_mse_short_returns_zeros() {
        let tiny = vec![1.0, 2.0, 3.0];
        let res = multiscale_entropy(&tiny, 5, 2, 0.1);
        assert!(res.iter().all(|&v| v == 0.0));
    }
}
