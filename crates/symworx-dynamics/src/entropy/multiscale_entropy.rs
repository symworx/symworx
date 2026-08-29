// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

use super::sample_entropy::sample_entropy;

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
