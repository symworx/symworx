// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Sample entropy, matching `symworx-dynamics`.
//!
//! Templates of length `m` start at `0..n-m`. Auto-entropy pairs are `i < j`.
//! A template matches at `m` unless some lag has `abs(diff) > r`. The `m + 1`
//! count also needs the next lag `<= r`. The result is `-ln(A/B)` in nats.
//!
//! Returns 0 when fewer than two templates fit, the population SD is below
//! `1e-12`, `r` is below `1e-10`, or either count is zero. A non-finite SD
//! does not take that exit. A NaN difference fails `> r`, so it still counts
//! at `m`, and fails `<= r`, so it does not count at `m + 1`.
//!
//! [`cross_sample_entropy`] uses that match on every pair of templates between
//! two series, shared index included. One `r` makes the swapped call equal.

use crate::{
    event_window::population_std,
    float,
};

const STD_FLOOR: f64 = 1e-12;
const R_FLOOR: f64 = 1e-10;

/// Sample entropy of one series, in nats.
///
/// `m` is the embedding length. `r` is the tolerance, already scaled. The
/// event window passes `0.2` times the population standard deviation.
pub fn sample_entropy(data: &[f64], m: usize, r: f64) -> f64 {
    let Some(templates) = data.len().checked_sub(m).filter(|n| *n >= 2) else {
        return 0.0;
    };
    let sd = population_std(data);
    if sd < STD_FLOOR || r < R_FLOOR {
        return 0.0;
    }

    let (count_m, count_m1) = count_pairs(data, templates, m, r);
    if count_m == 0 || count_m1 == 0 {
        return 0.0;
    }
    -float::ln((count_m1 as f64) / (count_m as f64))
}

fn count_pairs(data: &[f64], templates: usize, m: usize, r: f64) -> (u64, u64) {
    let mut count_m = 0u64;
    let mut count_m1 = 0u64;
    for i in 0..templates - 1 {
        for j in (i + 1)..templates {
            let mut match_m = true;
            for k in 0..m {
                if (data[i + k] - data[j + k]).abs() > r {
                    match_m = false;
                    break;
                }
            }
            if match_m {
                count_m += 1;
                if (data[i + m] - data[j + m]).abs() <= r {
                    count_m1 += 1;
                }
            }
        }
    }
    (count_m, count_m1)
}

/// Cross-sample entropy of two series, in nats.
///
/// `r` is already scaled. The joint window passes `0.2` times the population
/// SD of the two trimmed series concatenated. Returns 0 when either side has
/// no template (`len <= m`), `r` is below `1e-10`, or either count is zero.
/// A constant series is still scored. NaN lags follow [`sample_entropy`].
pub fn cross_sample_entropy(x: &[f64], y: &[f64], m: usize, r: f64) -> f64 {
    let Some(templates_x) = x.len().checked_sub(m).filter(|n| *n >= 1) else {
        return 0.0;
    };
    let Some(templates_y) = y.len().checked_sub(m).filter(|n| *n >= 1) else {
        return 0.0;
    };
    if r < R_FLOOR {
        return 0.0;
    }

    let (count_m, count_m1) = cross_counts(x, y, templates_x, templates_y, m, r);
    if count_m == 0 || count_m1 == 0 {
        return 0.0;
    }
    -float::ln((count_m1 as f64) / (count_m as f64))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_constant_and_tiny_tolerance_are_zero() {
        assert_eq!(sample_entropy(&[1.0, 2.0, 3.0], 2, 0.5), 0.0);
        assert_eq!(sample_entropy(&[5.0; 20], 2, 0.1), 0.0);
        assert_eq!(sample_entropy(&[1.0, 2.0, 3.0, 4.0], 2, 1e-12), 0.0);
        assert_eq!(sample_entropy(&[0.0, 1.0, 2.0, 3.0], 1, 0.1), 0.0);
        assert_eq!(sample_entropy(&[1.0, 2.0], usize::MAX, 0.5), 0.0);
    }

    #[test]
    fn hand_counted_pairs_match() {
        // m = 1, r = 0.1. Three pairs match at m and one of those matches at m+1.
        let data = [0.0, 0.0, 0.05, 1.0, 0.0];
        assert_eq!(sample_entropy(&data, 1, 0.1), -float::ln(1.0 / 3.0));

        // m = 2. (0, 2) matches both lengths. (1, 3) matches m and fails m+1.
        let data = [0.0, 1.0, 0.0, 1.0, 0.05, 0.0];
        assert_eq!(sample_entropy(&data, 2, 0.1), -float::ln(0.5));
    }

    #[test]
    fn zero_embedding_counts_every_pair_then_the_raw_gap() {
        // The m loop is empty, so B is every pair. A keeps |x[i] - x[j]| <= r.
        let data = [0.0, 0.5, 0.05];
        assert_eq!(sample_entropy(&data, 0, 0.1), -float::ln(1.0 / 3.0));
    }

    #[test]
    fn a_nan_in_the_series_still_counts_finite_pairs() {
        // Population SD is NaN, so this does not take the near-constant exit.
        // (0, 2) matches both lengths. (1, 3) matches m and fails on the NaN.
        let data = [0.0, 1.0, 0.0, 1.0, f64::NAN];
        assert_eq!(sample_entropy(&data, 1, 0.2), -float::ln(0.5));
    }

    #[test]
    fn cross_sample_entropy_matches_a_hand_count() {
        let x = [0.0, 1.0, 0.0, 1.0, 0.0];
        let y = [0.0, 1.0, 0.0, 1.0, 1.0];
        let got = cross_sample_entropy(&x, &y, 2, 0.1);
        assert_eq!(got, -float::ln(3.0 / 5.0));
        assert_eq!(got, cross_sample_entropy(&y, &x, 2, 0.1));
    }

    #[test]
    fn cross_sample_entropy_counts_the_diagonal_that_sample_entropy_skips() {
        let x = [0.0, 0.0, 1.0, 0.0];
        assert_eq!(sample_entropy(&x, 1, 0.1), 0.0);
        assert_eq!(cross_sample_entropy(&x, &x, 1, 0.1), -float::ln(3.0 / 5.0));
    }

    #[test]
    fn cross_sample_entropy_early_exits_do_not_treat_a_constant_as_zero() {
        assert_eq!(cross_sample_entropy(&[0.0, 1.0], &[0.0, 1.0], 2, 0.5), 0.0);
        assert_eq!(cross_sample_entropy(&[0.0, 1.0, 2.0], &[0.0, 1.0, 2.0], 1, 1e-12), 0.0);
        assert_eq!(cross_sample_entropy(&[0.0, 1.0, 2.0], &[10.0, 11.0, 12.0], 1, 0.1), 0.0);
        let x = [1.0, 1.0, 1.0, 1.0];
        let y = [1.0, 1.0, 2.0, 1.0];
        assert_eq!(cross_sample_entropy(&x, &y, 1, 0.1), -float::ln(0.5));
    }

    #[test]
    fn cross_sample_entropy_counts_a_nan_lag_as_m_only() {
        let x = [0.0, 0.0, f64::NAN];
        let y = [0.0, 0.0, 0.0];
        assert_eq!(cross_sample_entropy(&x, &y, 1, 0.2), -float::ln(0.5));
    }
}
