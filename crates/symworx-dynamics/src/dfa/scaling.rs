// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Log-log slope of the fluctuation curve.

use super::DfaError;

/// One log-log fit, `ln F ≈ intercept + alpha * ln n`.
#[derive(Debug, Clone, PartialEq)]
pub struct DfaFit {
    /// Box lengths that entered this fit, sorted and unique.
    pub scales: Vec<usize>,
    /// Slope of `ln F(n)` against `ln n`.
    pub alpha: f64,
    /// Natural-log intercept of that line (the value of `ln F` at `n = 1`).
    pub intercept: f64,
}

/// Ordinary least-squares slope and intercept of `ln F` against `ln n`.
///
/// The pair is `(alpha, intercept)`, with `ln F ≈ intercept + alpha * ln n`
/// (natural log). `F` must be positive. Identical scales have no slope.
pub fn scaling_exponent(scales: &[usize], fluctuation: &[f64]) -> Result<(f64, f64), DfaError> {
    if scales.is_empty() {
        return Err(DfaError::Empty);
    }
    if scales.len() != fluctuation.len() {
        return Err(DfaError::LengthMismatch);
    }
    if scales.len() < 2 {
        return Err(DfaError::TooFewScales);
    }
    if scales.contains(&0) {
        return Err(DfaError::InvalidScale);
    }
    if fluctuation.iter().any(|value| !value.is_finite()) {
        return Err(DfaError::NonFinite);
    }
    if fluctuation.iter().any(|value| *value <= 0.0) {
        return Err(DfaError::NonPositiveFluctuation);
    }

    let mut mean_x = 0.0;
    let mut mean_y = 0.0;
    let mut xs = Vec::with_capacity(scales.len());
    let mut ys = Vec::with_capacity(fluctuation.len());
    for (&scale, &value) in scales.iter().zip(fluctuation.iter()) {
        let x = (scale as f64).ln();
        let y = value.ln();
        mean_x += x;
        mean_y += y;
        xs.push(x);
        ys.push(y);
    }
    let n = scales.len() as f64;
    mean_x /= n;
    mean_y /= n;

    let mut var_x = 0.0;
    let mut cov = 0.0;
    for i in 0..scales.len() {
        let dx = xs[i] - mean_x;
        var_x += dx * dx;
        cov += dx * (ys[i] - mean_y);
    }
    if var_x <= 1e-18 {
        return Err(DfaError::SingularFit);
    }
    let alpha = cov / var_x;
    let intercept = mean_y - alpha * mean_x;
    Ok((alpha, intercept))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(got: f64, expect: f64) {
        assert!((got - expect).abs() <= 1e-12, "{got} != {expect}");
    }

    #[test]
    fn scaling_exponent_rejects_bad_pairs() {
        assert_eq!(scaling_exponent(&[], &[]), Err(DfaError::Empty));
        assert_eq!(scaling_exponent(&[4], &[1.0, 2.0]), Err(DfaError::LengthMismatch));
        assert_eq!(scaling_exponent(&[4], &[1.0]), Err(DfaError::TooFewScales));
        assert_eq!(scaling_exponent(&[0, 8], &[1.0, 2.0]), Err(DfaError::InvalidScale));
        assert_eq!(scaling_exponent(&[4, 8], &[1.0, f64::NAN]), Err(DfaError::NonFinite));
        assert_eq!(
            scaling_exponent(&[4, 8], &[1.0, 0.0]),
            Err(DfaError::NonPositiveFluctuation)
        );
        assert_eq!(scaling_exponent(&[4, 4], &[1.0, 2.0]), Err(DfaError::SingularFit));
    }

    #[test]
    fn scaling_exponent_of_a_square_root_law_is_one_half() {
        // F(4) = 2, F(16) = 4, so F ∝ n^0.5 and the line passes through ln F = 0 at n = 1.
        let (alpha, intercept) = scaling_exponent(&[4, 16], &[2.0, 4.0]).unwrap();
        assert_close(alpha, 0.5);
        assert_close(intercept, 0.0);
    }
}
