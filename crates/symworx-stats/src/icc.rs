// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Shrout–Fleiss ICC(1,1) for two replicates per subject.
//!
//! One-way random, single measure, `k = 2`. A shift between the two
//! replicates counts as disagreement. A negative coefficient is a valid
//! result.
//!
//! Study reports that leave the coefficient blank below a larger pair count
//! (for example three) apply that rule at the call site.

use std::fmt;

/// Errors from [`icc_11`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IccError {
    /// Fewer than two subject pairs.
    TooFewPairs {
        /// Number of pairs supplied.
        n: usize,
    },
}

impl fmt::Display for IccError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IccError::TooFewPairs { n } => {
                write!(f, "ICC(1,1) needs at least 2 pairs, got {n}")
            }
        }
    }
}

impl std::error::Error for IccError {}

/// Shrout–Fleiss ICC(1,1) for `n` subjects and two replicates.
///
/// Each pair is `(y_i1, y_i2)`.
///
/// ```text
/// BMS = 2 * Σ (m_i - m)^2 / (n - 1)
/// WMS = Σ (y_i1 - y_i2)^2 / (2n)
/// ICC = (BMS - WMS) / (BMS + WMS)
/// ```
///
/// `m_i` is the mean of the two replicates and `m` is the mean of the `m_i`.
/// When both mean squares are 0, every replicate equals the same value, so
/// the coefficient is 1. At least two pairs are required.
pub fn icc_11(pairs: &[(f64, f64)]) -> Result<f64, IccError> {
    let n = pairs.len();
    if n < 2 {
        return Err(IccError::TooFewPairs { n });
    }

    let n_f = n as f64;
    let mut mean_sum = 0.0;
    for &(y1, y2) in pairs {
        mean_sum += (y1 + y2) / 2.0;
    }
    let grand = mean_sum / n_f;

    let mut between = 0.0;
    let mut within = 0.0;
    for &(y1, y2) in pairs {
        let mean = (y1 + y2) / 2.0;
        let mean_dev = mean - grand;
        between += mean_dev * mean_dev;
        let replicate_dev = y1 - y2;
        within += replicate_dev * replicate_dev;
    }

    let bms = 2.0 * between / (n_f - 1.0);
    let wms = within / (2.0 * n_f);
    if bms == 0.0 && wms == 0.0 {
        return Ok(1.0);
    }
    Ok((bms - wms) / (bms + wms))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_matching_pairs_are_one() {
        // Between-subject spread with perfect replicate agreement.
        assert_eq!(icc_11(&[(1.0, 1.0), (3.0, 3.0)]).unwrap(), 1.0);
    }

    #[test]
    fn repeated_within_pair_disagreement_is_minus_one() {
        assert_eq!(icc_11(&[(1.0, 3.0), (1.0, 3.0)]).unwrap(), -1.0);
    }

    #[test]
    fn identical_values_are_one_when_both_mean_squares_are_zero() {
        assert_eq!(icc_11(&[(4.0, 4.0), (4.0, 4.0), (4.0, 4.0)]).unwrap(), 1.0);
    }

    #[test]
    fn constant_offset_between_replicates_is_zero() {
        // Subject means differ, and every second replicate is +2.
        assert_eq!(icc_11(&[(1.0, 3.0), (2.0, 4.0), (3.0, 5.0)]).unwrap(), 0.0);
    }

    #[test]
    fn unequal_replicates_match_the_mean_square_formula() {
        // means 1.5 and 4.5; BMS = 9; WMS = 2.5
        let icc = icc_11(&[(1.0, 2.0), (3.0, 6.0)]).unwrap();
        assert!((icc - (6.5 / 11.5)).abs() < 1e-12);
    }

    #[test]
    fn fewer_than_two_pairs_is_an_error() {
        assert_eq!(icc_11(&[]), Err(IccError::TooFewPairs { n: 0 }));
        assert_eq!(icc_11(&[(1.0, 1.0)]), Err(IccError::TooFewPairs { n: 1 }));
        assert_eq!(
            icc_11(&[(1.0, 2.0)]).unwrap_err().to_string(),
            "ICC(1,1) needs at least 2 pairs, got 1"
        );
    }
}
