// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Box detrending and the root-mean-square fluctuation `F(n)`.

use super::{
    DfaError,
    check_order,
    check_scale,
    check_series,
};

/// Root-mean-square fluctuation of `profile` at one box length.
///
/// Boxes of length `scale` tile the profile from the start and again from the
/// end (Kantelhardt). In each box a polynomial of degree `order` is removed.
/// `F` is the root mean square of those residuals across both passes. A box
/// must hold at least `order + 2` samples.
pub fn fluctuation(profile: &[f64], scale: usize, order: usize) -> Result<f64, DfaError> {
    check_series(profile)?;
    check_order(order)?;
    check_scale(scale, order, profile.len())?;

    let n_seg = profile.len() / scale;
    let mut sum_sq = 0.0;
    for segment in profile.chunks_exact(scale) {
        sum_sq += residual_sum_sq(segment, order)?;
    }
    for seg in 0..n_seg {
        let start = profile.len() - (seg + 1) * scale;
        sum_sq += residual_sum_sq(&profile[start..start + scale], order)?;
    }
    let denom = (2 * n_seg * scale) as f64;
    Ok((sum_sq / denom).sqrt())
}

fn residual_sum_sq(segment: &[f64], order: usize) -> Result<f64, DfaError> {
    if order == 1 {
        return Ok(linear_residual_sum_sq(segment));
    }
    polynomial_residual_sum_sq(segment, order)
}

/// Sum of squared residuals about the least-squares line.
///
/// The time index is `0 .. n-1`. `sum (t - mean_t)^2 = n (n^2 - 1) / 12`.
fn linear_residual_sum_sq(segment: &[f64]) -> f64 {
    let n = segment.len() as f64;
    let mean_t = (n - 1.0) / 2.0;
    let mean_y = segment.iter().sum::<f64>() / n;
    let mut cov = 0.0;
    for (i, &sample) in segment.iter().enumerate() {
        cov += (i as f64 - mean_t) * (sample - mean_y);
    }
    let sum_sq_t = n * (n * n - 1.0) / 12.0;
    let slope = cov / sum_sq_t;
    let intercept = mean_y - slope * mean_t;
    let mut sum_sq = 0.0;
    for (i, &sample) in segment.iter().enumerate() {
        let residual = sample - (intercept + slope * i as f64);
        sum_sq += residual * residual;
    }
    sum_sq
}

/// Sum of squared residuals about a degree-`order` polynomial.
///
/// The basis is centered and scaled onto `[-1, 1]`, so a high power does not
/// overflow a long box. The column space is still the polynomials of that degree.
fn polynomial_residual_sum_sq(segment: &[f64], order: usize) -> Result<f64, DfaError> {
    let n = segment.len();
    let p = order + 1;
    let half = (n as f64 - 1.0) / 2.0;
    let mut ata = vec![0.0; p * p];
    let mut aty = vec![0.0; p];
    for (i, &sample) in segment.iter().enumerate() {
        let t = (i as f64 - half) / half;
        let mut power = 1.0;
        let mut powers = vec![0.0; p];
        for term in &mut powers {
            *term = power;
            power *= t;
        }
        for row in 0..p {
            aty[row] += powers[row] * sample;
            for col in 0..p {
                ata[row * p + col] += powers[row] * powers[col];
            }
        }
    }
    let coeff = solve(ata, aty)?;
    let mut sum_sq = 0.0;
    for (i, &sample) in segment.iter().enumerate() {
        let t = (i as f64 - half) / half;
        let mut power = 1.0;
        let mut fitted = 0.0;
        for &weight in &coeff {
            fitted += weight * power;
            power *= t;
        }
        let residual = sample - fitted;
        sum_sq += residual * residual;
    }
    Ok(sum_sq)
}

/// Solve `A x = b` for a square row-major `A`. Partial pivoting.
fn solve(mut ata: Vec<f64>, mut aty: Vec<f64>) -> Result<Vec<f64>, DfaError> {
    let p = aty.len();
    let scale = ata.iter().fold(0.0_f64, |max, value| max.max(value.abs())).max(1.0);
    for col in 0..p {
        let mut pivot_row = col;
        let mut pivot_abs = ata[col * p + col].abs();
        for row in (col + 1)..p {
            let value = ata[row * p + col].abs();
            if value > pivot_abs {
                pivot_abs = value;
                pivot_row = row;
            }
        }
        if pivot_abs <= scale * 1e-12 {
            return Err(DfaError::SingularFit);
        }
        if pivot_row != col {
            for k in col..p {
                ata.swap(col * p + k, pivot_row * p + k);
            }
            aty.swap(col, pivot_row);
        }
        let pivot = ata[col * p + col];
        for row in (col + 1)..p {
            let factor = ata[row * p + col] / pivot;
            for k in col..p {
                ata[row * p + k] -= factor * ata[col * p + k];
            }
            aty[row] -= factor * aty[col];
        }
    }
    let mut coeff = vec![0.0; p];
    for row in (0..p).rev() {
        let mut value = aty[row];
        for k in (row + 1)..p {
            value -= ata[row * p + k] * coeff[k];
        }
        let diag = ata[row * p + row];
        if diag.abs() <= scale * 1e-12 {
            return Err(DfaError::SingularFit);
        }
        coeff[row] = value / diag;
    }
    Ok(coeff)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(got: f64, expect: f64) {
        assert!((got - expect).abs() <= 1e-12, "{got} != {expect}");
    }

    #[test]
    fn fluctuation_rejects_bad_input() {
        let profile = [0.0, 1.0, 2.0, 1.0, 0.0, 1.0];
        assert_eq!(fluctuation(&[], 4, 1), Err(DfaError::Empty));
        assert_eq!(fluctuation(&[0.0, f64::NAN, 1.0, 1.0], 4, 1), Err(DfaError::NonFinite));
        assert_eq!(fluctuation(&profile, 4, 0), Err(DfaError::InvalidOrder));
        assert_eq!(fluctuation(&profile, 2, 1), Err(DfaError::InvalidScale));
        assert_eq!(fluctuation(&profile, 8, 1), Err(DfaError::InvalidScale));
    }

    #[test]
    fn linear_detrend_of_a_line_is_zero_and_a_step_matches_a_hand_fit() {
        assert_close(fluctuation(&[0.0, 1.0, 2.0, 3.0, 4.0, 5.0], 3, 1).unwrap(), 0.0);
        assert_close(fluctuation(&[0.0, 0.0, 0.0, 0.0], 4, 1).unwrap(), 0.0);
        // One box, used from both ends. Residuals about the line sum to 1/5
        // per pass, so F^2 = (2/5) / 8.
        assert_close(fluctuation(&[0.0, 0.0, 1.0, 1.0], 4, 1).unwrap(), (0.05_f64).sqrt());
    }

    #[test]
    fn backward_pass_includes_the_tail_a_forward_pass_drops() {
        // Forward box [0, 0, 0] has no residual. The tail [0, 1, 1] does.
        // F = 1/6 when that tail is included once next to the zero box.
        assert_close(fluctuation(&[0.0, 0.0, 0.0, 1.0, 1.0], 3, 1).unwrap(), 1.0 / 6.0);
    }

    #[test]
    fn polynomial_order_removes_that_degree_and_leaves_the_next() {
        let quadratic: Vec<f64> = (0..8)
            .map(|i| {
                let t = i as f64 - 3.5;
                t * t
            })
            .collect();
        assert!(fluctuation(&quadratic, 8, 2).unwrap() < 1e-9);
        assert!(fluctuation(&quadratic, 8, 1).unwrap() > 1e-3);

        let cubic: Vec<f64> = (0..8)
            .map(|i| {
                let t = i as f64 - 3.5;
                t * t * t
            })
            .collect();
        assert!(fluctuation(&cubic, 8, 3).unwrap() < 1e-8);
        assert!(fluctuation(&cubic, 8, 2).unwrap() > 1e-3);
    }
}
