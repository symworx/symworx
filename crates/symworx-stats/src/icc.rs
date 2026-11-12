// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Shrout–Fleiss intraclass correlation for a balanced subjects-by-replicates table.
//!
//! Columns are matched occasions: column `j` is the same replicate for every
//! subject. [`icc`] returns the six coefficients from one ANOVA.
//!
//! * ICC(1,1) and ICC(1,k) are one-way random. A column shift stays in the
//!   within-subject mean square.
//! * ICC(2,1) and ICC(2,k) are two-way random, absolute agreement. A column
//!   shift counts against agreement.
//! * ICC(3,1) and ICC(3,k) are two-way mixed, consistency. An additive column
//!   shift is removed before the coefficient is formed.
//!
//! [`icc_11`] is ICC(1,1) for exactly two replicates. A negative coefficient
//! is a valid result. When every entry is the same value, each coefficient
//! is 1. A zero denominator on any other table is `NaN`.
//!
//! Study reports that leave a coefficient blank below a larger subject count
//! apply that rule at the call site. These functions are for a complete
//! rectangle. A subject with a different number of replicates is an error.

use std::fmt;

/// Errors from [`icc`] and [`icc_11`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IccError {
    /// Fewer than two subject pairs in [`icc_11`].
    TooFewPairs {
        /// Number of pairs supplied.
        n: usize,
    },
    /// Fewer than two subjects.
    TooFewSubjects {
        /// Number of rows supplied.
        n: usize,
    },
    /// Fewer than two replicates.
    TooFewColumns {
        /// Number of columns supplied.
        k: usize,
    },
    /// A row length differs from the first row.
    RaggedRows {
        /// Length of the first row.
        expected: usize,
        /// Index of the row that differs.
        row: usize,
        /// Length of that row.
        len: usize,
    },
}

impl fmt::Display for IccError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IccError::TooFewPairs { n } => {
                write!(f, "ICC(1,1) needs at least 2 pairs, got {n}")
            }
            IccError::TooFewSubjects { n } => {
                write!(f, "ICC needs at least 2 subjects, got {n}")
            }
            IccError::TooFewColumns { k } => {
                write!(f, "ICC needs at least 2 columns, got {k}")
            }
            IccError::RaggedRows { expected, row, len } => {
                write!(f, "ICC row {row} has length {len}; expected {expected}")
            }
        }
    }
}

impl std::error::Error for IccError {}

/// Six Shrout–Fleiss coefficients for one balanced table.
///
/// `n` is the number of subjects and `k` is the number of replicates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Icc {
    /// Number of subjects (rows).
    pub n: usize,
    /// Number of matched replicates (columns).
    pub k: usize,
    /// One-way random, single replicate.
    pub icc_11: f64,
    /// One-way random, mean of the `k` replicates.
    pub icc_1k: f64,
    /// Two-way random, absolute agreement, single replicate.
    pub icc_21: f64,
    /// Two-way random, absolute agreement, mean of the `k` replicates.
    pub icc_2k: f64,
    /// Two-way mixed, consistency, single replicate.
    pub icc_31: f64,
    /// Two-way mixed, consistency, mean of the `k` replicates.
    pub icc_3k: f64,
}

/// Shrout–Fleiss coefficients for a subjects-by-replicates table.
///
/// Each row is one subject. Rows must share one length `k >= 2`, and at
/// least two subjects are required.
///
/// ```text
/// ICC(1,1) = (BMS - WMS) / (BMS + (k - 1) WMS)
/// ICC(1,k) = (BMS - WMS) / BMS
/// ICC(2,1) = (BMS - EMS) / (BMS + (k - 1) EMS + k (JMS - EMS) / n)
/// ICC(2,k) = (BMS - EMS) / (BMS + (JMS - EMS) / n)
/// ICC(3,1) = (BMS - EMS) / (BMS + (k - 1) EMS)
/// ICC(3,k) = (BMS - EMS) / BMS
/// ```
///
/// `BMS` is between subjects, `WMS` is within subjects, `JMS` is between
/// columns, and `EMS` is the residual after subject and column means.
pub fn icc<R: AsRef<[f64]>>(rows: &[R]) -> Result<Icc, IccError> {
    let n = rows.len();
    if n < 2 {
        return Err(IccError::TooFewSubjects { n });
    }
    let k = rows[0].as_ref().len();
    for (row, values) in rows.iter().enumerate().skip(1) {
        let len = values.as_ref().len();
        if len != k {
            return Err(IccError::RaggedRows { expected: k, row, len });
        }
    }
    if k < 2 {
        return Err(IccError::TooFewColumns { k });
    }

    let n_f = n as f64;
    let k_f = k as f64;
    let mut row_means = Vec::with_capacity(n);
    let mut grand_sum = 0.0;
    for row in rows {
        let sum: f64 = row.as_ref().iter().copied().sum();
        grand_sum += sum;
        row_means.push(sum / k_f);
    }
    let grand = grand_sum / (n_f * k_f);

    let mut col_sums = vec![0.0; k];
    for row in rows {
        for (j, &y) in row.as_ref().iter().enumerate() {
            col_sums[j] += y;
        }
    }
    let col_means: Vec<f64> = col_sums.into_iter().map(|sum| sum / n_f).collect();

    let mut ss_rows = 0.0;
    for &mean in &row_means {
        let dev = mean - grand;
        ss_rows += dev * dev;
    }
    ss_rows *= k_f;

    let mut ss_cols = 0.0;
    for &mean in &col_means {
        let dev = mean - grand;
        ss_cols += dev * dev;
    }
    ss_cols *= n_f;

    let mut ss_within = 0.0;
    let mut ss_error = 0.0;
    for (i, row) in rows.iter().enumerate() {
        for (j, &y) in row.as_ref().iter().enumerate() {
            let within = y - row_means[i];
            ss_within += within * within;
            let residual = y - row_means[i] - col_means[j] + grand;
            ss_error += residual * residual;
        }
    }

    let bms = ss_rows / (n_f - 1.0);
    let wms = ss_within / (n_f * (k_f - 1.0));
    let jms = ss_cols / (k_f - 1.0);
    let ems = ss_error / ((n_f - 1.0) * (k_f - 1.0));
    let all_equal = bms == 0.0 && wms == 0.0;
    let judge = (jms - ems) / n_f;

    Ok(Icc {
        n,
        k,
        icc_11: coefficient(bms - wms, bms + (k_f - 1.0) * wms, all_equal),
        icc_1k: coefficient(bms - wms, bms, all_equal),
        icc_21: coefficient(bms - ems, bms + (k_f - 1.0) * ems + k_f * judge, all_equal),
        icc_2k: coefficient(bms - ems, bms + judge, all_equal),
        icc_31: coefficient(bms - ems, bms + (k_f - 1.0) * ems, all_equal),
        icc_3k: coefficient(bms - ems, bms, all_equal),
    })
}

/// Shrout–Fleiss ICC(1,1) for `n` subjects and two replicates.
///
/// Each pair is `(y_i1, y_i2)`. This is [`icc`] with `k = 2`, returning
/// only the one-way single-replicate coefficient.
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
    let rows: Vec<[f64; 2]> = pairs.iter().map(|&(y1, y2)| [y1, y2]).collect();
    Ok(icc(&rows)?.icc_11)
}

fn coefficient(numerator: f64, denominator: f64, all_equal: bool) -> f64 {
    if all_equal {
        1.0
    } else if denominator == 0.0 {
        f64::NAN
    } else {
        numerator / denominator
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
    }

    #[test]
    fn two_matching_pairs_are_one() {
        assert_eq!(icc_11(&[(1.0, 1.0), (3.0, 3.0)]).unwrap(), 1.0);
        let fit = icc(&[[1.0, 1.0], [3.0, 3.0]]).unwrap();
        assert_eq!(fit.n, 2);
        assert_eq!(fit.k, 2);
        assert_eq!(fit.icc_11, 1.0);
        assert_eq!(fit.icc_1k, 1.0);
        assert_eq!(fit.icc_21, 1.0);
        assert_eq!(fit.icc_2k, 1.0);
        assert_eq!(fit.icc_31, 1.0);
        assert_eq!(fit.icc_3k, 1.0);
    }

    #[test]
    fn repeated_within_pair_disagreement_is_minus_one() {
        assert_eq!(icc_11(&[(1.0, 3.0), (1.0, 3.0)]).unwrap(), -1.0);
        let fit = icc(&[[1.0, 3.0], [1.0, 3.0]]).unwrap();
        assert_eq!(fit.icc_11, -1.0);
        assert_eq!(fit.icc_21, 0.0);
        assert_eq!(fit.icc_2k, 0.0);
        assert!(fit.icc_1k.is_nan());
        assert!(fit.icc_31.is_nan());
        assert!(fit.icc_3k.is_nan());
    }

    #[test]
    fn identical_values_are_one_when_both_mean_squares_are_zero() {
        assert_eq!(icc_11(&[(4.0, 4.0), (4.0, 4.0), (4.0, 4.0)]).unwrap(), 1.0);
        let fit = icc(&[[4.0, 4.0], [4.0, 4.0], [4.0, 4.0]]).unwrap();
        assert_eq!(fit.icc_11, 1.0);
        assert_eq!(fit.icc_1k, 1.0);
        assert_eq!(fit.icc_21, 1.0);
        assert_eq!(fit.icc_2k, 1.0);
        assert_eq!(fit.icc_31, 1.0);
        assert_eq!(fit.icc_3k, 1.0);
    }

    #[test]
    fn constant_offset_between_replicates_is_zero_for_one_way() {
        // Subject means differ, and every second replicate is +2.
        assert_eq!(icc_11(&[(1.0, 3.0), (2.0, 4.0), (3.0, 5.0)]).unwrap(), 0.0);
        let fit = icc(&[[1.0, 3.0], [2.0, 4.0], [3.0, 5.0]]).unwrap();
        assert_eq!(fit.icc_11, 0.0);
        assert_eq!(fit.icc_1k, 0.0);
        close(fit.icc_21, 1.0 / 3.0);
        assert_eq!(fit.icc_2k, 0.5);
        assert_eq!(fit.icc_31, 1.0);
        assert_eq!(fit.icc_3k, 1.0);
    }

    #[test]
    fn unequal_replicates_match_the_mean_square_formula() {
        // means 1.5 and 4.5; BMS = 9; WMS = 2.5
        let icc = icc_11(&[(1.0, 2.0), (3.0, 6.0)]).unwrap();
        close(icc, 6.5 / 11.5);
    }

    #[test]
    fn three_replicates_separate_the_column_term() {
        // y_ij = subject + replicate, so the residual is 0.
        let fit = icc(&[[1.0, 2.0, 3.0], [2.0, 3.0, 4.0], [3.0, 4.0, 5.0]]).unwrap();
        assert_eq!(fit.k, 3);
        close(fit.icc_11, 0.4);
        close(fit.icc_1k, 2.0 / 3.0);
        assert_eq!(fit.icc_21, 0.5);
        assert_eq!(fit.icc_2k, 0.75);
        assert_eq!(fit.icc_31, 1.0);
        assert_eq!(fit.icc_3k, 1.0);
    }

    #[test]
    fn crossed_replicates_can_push_the_average_agreement_above_one() {
        let fit = icc(&[[1.0, 10.0], [10.0, 1.0]]).unwrap();
        assert_eq!(fit.icc_11, -1.0);
        assert_eq!(fit.icc_31, -1.0);
        assert_eq!(fit.icc_2k, 2.0);
        assert!(fit.icc_21.is_nan());
        assert!(fit.icc_1k.is_nan());
        assert!(fit.icc_3k.is_nan());
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

    #[test]
    fn table_shape_is_checked() {
        assert_eq!(icc(&[[1.0, 2.0]]), Err(IccError::TooFewSubjects { n: 1 }));
        assert_eq!(icc(&[[1.0], [2.0]]), Err(IccError::TooFewColumns { k: 1 }));
        assert_eq!(
            icc(&[vec![1.0, 2.0], vec![3.0]]),
            Err(IccError::RaggedRows {
                expected: 2,
                row: 1,
                len: 1
            })
        );
    }
}
