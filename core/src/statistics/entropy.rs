// src/statistics/entropy.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use ndarray::prelude::*;

/// Calculate Sample Entropy (SampEn) of a time series.
///
/// # Arguments
/// * `data` - Input time series
/// * `m`    - Embedding dimension (typically 2)
/// * `r`    - Tolerance (similarity threshold)
/// 
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
}
