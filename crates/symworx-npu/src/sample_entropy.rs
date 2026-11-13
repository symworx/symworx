// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Sample entropy.
//!
//! This matches `symworx-dynamics`. Windows of length `m` start at `0..n-m`.
//! Pairs use `i < j`. A pair matches at `m` unless some lag has
//! `abs(x[i + k] - x[j + k]) > r`. The `m + 1` count also requires
//! `abs(x[i + m] - x[j + m]) <= r`. The result is `-ln(A / B)`, in nats.
//!
//! The result is 0 when fewer than two windows fit, the population standard
//! deviation is below `1e-12`, `r` is below `1e-10`, or either count is zero.
//! A non-finite standard deviation does not take that exit. A NaN difference
//! does not satisfy `> r`, so it still counts as an `m` match. The extra lag
//! uses `<= r`, so a NaN there does not count as `m + 1`.

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
}
