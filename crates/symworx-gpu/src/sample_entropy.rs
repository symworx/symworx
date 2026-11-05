// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Sample entropy counts.
//!
//! This matches `symworx_dynamics::sample_entropy`, including its loop bounds.
//! Windows of length `m` start at `0..n-m`. Pairs use `i < j`. A pair matches
//! at `m` unless some lag has `abs(x[i+k] - x[j+k]) > r`. The `m+1` count also
//! requires `abs(x[i+m] - x[j+m]) <= r`. The result is `-ln(A/B)`.
//!
//! A NaN difference does not satisfy `> r`, so it still counts as an `m` match.
//! The extra lag uses `<= r`, so a NaN there does not count as `m+1`.

use crate::error::GpuError;

const STD_FLOOR: f64 = 1e-12;
const R_FLOOR: f64 = 1e-10;

/// CPU sample entropy. This is the specification the device kernel matches.
pub fn sample_entropy_cpu(data: &[f64], m: usize, r: f64) -> Result<f64, GpuError> {
    if !needs_counts(data, m, r)? {
        return Ok(0.0);
    }
    let (count_m, count_m1) = count_pairs(data, m, r);
    Ok(entropy_from_counts(count_m, count_m1))
}

/// Device sample entropy.
///
/// Without the `hip` feature this returns [`GpuError::Unavailable`]. With
/// `hip`, short series, a near-zero tolerance, and a near-constant series
/// return `0.0` before launch. There is no CPU fallback for the pair count.
pub fn sample_entropy(data: &[f64], m: usize, r: f64) -> Result<f64, GpuError> {
    #[cfg(not(feature = "hip"))]
    {
        let _ = (data, m, r);
        Err(GpuError::Unavailable)
    }
    #[cfg(feature = "hip")]
    {
        crate::device::sample_entropy_hip(data, m, r)
    }
}

/// `false` when the CPU function returns `0.0` before counting pairs.
pub fn needs_counts(data: &[f64], m: usize, r: f64) -> Result<bool, GpuError> {
    if data.len() > i32::MAX as usize || m > i32::MAX as usize {
        return Err(GpuError::InvalidInput("sample entropy length does not fit in i32"));
    }
    let n = data.len();
    if n <= m + 1 || population_std(data) < STD_FLOOR || r < R_FLOOR {
        return Ok(false);
    }
    Ok(true)
}

fn count_pairs(data: &[f64], m: usize, r: f64) -> (u64, u64) {
    let templates = data.len() - m;
    let mut count_m = 0u64;
    let mut count_m1 = 0u64;
    for i in 0..templates - 1 {
        for j in (i + 1)..templates {
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
                    count_m1 += 1;
                }
            }
        }
    }
    (count_m, count_m1)
}

pub fn entropy_from_counts(count_m: u64, count_m1: u64) -> f64 {
    if count_m == 0 || count_m1 == 0 {
        0.0
    } else {
        -((count_m1 as f64) / (count_m as f64)).ln()
    }
}

fn population_std(data: &[f64]) -> f64 {
    let n = data.len() as f64;
    let mut sum = 0.0;
    for value in data {
        sum += value;
    }
    let mean = sum / n;
    let mut var = 0.0;
    for value in data {
        let diff = value - mean;
        var += diff * diff;
    }
    (var / n).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_constant_and_tiny_tolerance_are_zero() {
        assert_eq!(sample_entropy_cpu(&[1.0, 2.0, 3.0], 2, 0.5).unwrap(), 0.0);
        assert_eq!(sample_entropy_cpu(&[5.0; 20], 2, 0.1).unwrap(), 0.0);
        assert_eq!(sample_entropy_cpu(&[1.0, 2.0, 3.0, 4.0], 2, 1e-12).unwrap(), 0.0);
    }

    #[test]
    fn hand_counted_pairs_match() {
        // m = 1, r = 0.1. Three pairs match at m and one of those matches at m+1.
        let data = [0.0, 0.0, 0.05, 1.0, 0.0];
        let got = sample_entropy_cpu(&data, 1, 0.1).unwrap();
        assert_eq!(got, -((1.0_f64) / 3.0).ln());

        // m = 2. (0,2) matches both lengths. (1,3) matches m and fails m+1.
        let data = [0.0, 1.0, 0.0, 1.0, 0.05, 0.0];
        let got = sample_entropy_cpu(&data, 2, 0.1).unwrap();
        assert_eq!(got, -((1.0_f64) / 2.0).ln());
    }

    #[test]
    fn zero_embedding_counts_every_pair_then_the_raw_gap() {
        // The m loop is empty, so B is every pair. A keeps |x[i] - x[j]| <= r.
        let data = [0.0, 0.5, 0.05];
        let got = sample_entropy_cpu(&data, 0, 0.1).unwrap();
        assert_eq!(got, -((1.0_f64) / 3.0).ln());
    }

    #[test]
    fn length_above_i32_is_rejected() {
        let err = sample_entropy_cpu(&[1.0], (i32::MAX as usize) + 1, 0.2).unwrap_err();
        assert!(matches!(err, GpuError::InvalidInput(_)));
    }

    #[cfg(not(feature = "hip"))]
    #[test]
    fn device_entry_is_unavailable_without_hip() {
        let err = sample_entropy(&[0.0, 1.0, 0.2, 1.1, 0.1], 2, 0.5).unwrap_err();
        assert!(matches!(err, GpuError::Unavailable));
    }
}
