// Copyright (c) 2026 Nathaniel T. Berry
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

/// Cross-sample entropy of two series, in nats.
///
/// Every template in `x` is compared with every template in `y`, including
/// the pair that shares an index. The same `r` makes the swapped call equal.
/// The calls differ only when the caller scales `r` from one of the series.
///
/// Returns 0 when either series is shorter than `m + 1`, `r` is below
/// `1e-10`, or either count is zero. A constant series is still scored.
/// A NaN lag counts at `m` and not at `m + 1`, as in [`sample_entropy`].
pub fn cross_sample_entropy(x: &[f64], y: &[f64], m: usize, r: f64) -> f64 {
    let Some(templates_x) = x.len().checked_sub(m).filter(|n| *n >= 1) else {
        return 0.0;
    };
    let Some(templates_y) = y.len().checked_sub(m).filter(|n| *n >= 1) else {
        return 0.0;
    };
    if r < 1e-10 {
        return 0.0;
    }

    let (count_m, count_m1) = cross_counts(x, y, templates_x, templates_y, m, r);
    if count_m == 0 || count_m1 == 0 {
        return 0.0;
    }
    -((count_m1 as f64) / (count_m as f64)).ln()
}

fn cross_counts(x: &[f64], y: &[f64], templates_x: usize, templates_y: usize, m: usize, r: f64) -> (u64, u64) {
    let mut count_m = 0u64;
    let mut count_m1 = 0u64;
    for i in 0..templates_x {
        for j in 0..templates_y {
            let mut match_m = true;
            for k in 0..m {
                if (x[i + k] - y[j + k]).abs() > r {
                    match_m = false;
                    break;
                }
            }
            if match_m {
                count_m += 1;
                if (x[i + m] - y[j + m]).abs() <= r {
                    count_m1 += 1;
                }
            }
        }
    }
    (count_m, count_m1)
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
    fn cross_sample_entropy_matches_a_hand_count() {
        // m = 2. Five pairs match at m and three of those match at m+1.
        let x = [0.0, 1.0, 0.0, 1.0, 0.0];
        let y = [0.0, 1.0, 0.0, 1.0, 1.0];
        let got = cross_sample_entropy(&x, &y, 2, 0.1);
        assert_eq!(got, -((3.0_f64) / 5.0).ln());
        assert_eq!(got, cross_sample_entropy(&y, &x, 2, 0.1));
    }

    #[test]
    fn cross_sample_entropy_counts_the_diagonal_that_sample_entropy_skips() {
        let x = [0.0, 0.0, 1.0, 0.0];
        assert_eq!(sample_entropy(&x, 1, 0.1), 0.0);
        assert_eq!(cross_sample_entropy(&x, &x, 1, 0.1), -((3.0_f64) / 5.0).ln());
    }

    #[test]
    fn cross_sample_entropy_early_exits_do_not_treat_a_constant_as_zero() {
        assert_eq!(cross_sample_entropy(&[0.0, 1.0], &[0.0, 1.0], 2, 0.5), 0.0);
        assert_eq!(cross_sample_entropy(&[0.0, 1.0, 2.0], &[0.0, 1.0, 2.0], 1, 1e-12), 0.0);
        assert_eq!(cross_sample_entropy(&[0.0, 1.0, 2.0], &[10.0, 11.0, 12.0], 1, 0.1), 0.0);

        // x is constant. y is not. Matches still count.
        let x = [1.0, 1.0, 1.0, 1.0];
        let y = [1.0, 1.0, 2.0, 1.0];
        assert_eq!(cross_sample_entropy(&x, &y, 1, 0.1), -((3.0_f64) / 6.0).ln());
    }

    #[test]
    fn cross_sample_entropy_counts_a_nan_lag_as_m_only() {
        let x = [0.0, 0.0, f64::NAN];
        let y = [0.0, 0.0, 0.0];
        assert_eq!(cross_sample_entropy(&x, &y, 1, 0.2), -((2.0_f64) / 4.0).ln());
    }
}
