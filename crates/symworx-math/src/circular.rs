// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Circular (angular) statistics.
//!
//! All angles are in **radians**. Wrapping uses the principal interval `(-π, π]`.
//! Empty or non-finite inputs return [`None`] rather than `NaN`.

use std::f64::consts::PI;

/// Wrap `angle` to `(-π, π]`.
///
/// Non-finite inputs are returned unchanged.
pub fn wrap_pi(angle: f64) -> f64 {
    if !angle.is_finite() {
        return angle;
    }
    let two_pi = 2.0 * PI;
    let mut a = angle.rem_euclid(two_pi);
    if a > PI {
        a -= two_pi;
    }
    a
}

/// Signed circular difference `wrap(a - b)` in `(-π, π]`.
pub fn angular_diff(a: f64, b: f64) -> f64 {
    wrap_pi(a - b)
}

fn finite_angles(angles: &[f64]) -> Option<Vec<(f64, f64)>> {
    if angles.is_empty() {
        return None;
    }
    let mut pairs = Vec::with_capacity(angles.len());
    for &a in angles {
        if !a.is_finite() {
            continue;
        }
        pairs.push((a.cos(), a.sin()));
    }
    if pairs.is_empty() { None } else { Some(pairs) }
}

/// Circular mean (resultant direction) of `angles`.
///
/// Returns [`None`] when there are no finite samples, or when the mean
/// resultant length is ~0 (no preferred direction).
pub fn circular_mean(angles: &[f64]) -> Option<f64> {
    let pairs = finite_angles(angles)?;
    let n = pairs.len() as f64;
    let mut c = 0.0;
    let mut s = 0.0;
    for (cos_a, sin_a) in pairs {
        c += cos_a;
        s += sin_a;
    }
    c /= n;
    s /= n;
    if (c * c + s * s).sqrt() < 1e-12 {
        return None;
    }
    Some(s.atan2(c))
}

/// Mean resultant length `R ∈ [0, 1]`.
///
/// `R = 1` when all finite angles agree; `R = 0` when they cancel.
pub fn mean_resultant_length(angles: &[f64]) -> Option<f64> {
    let pairs = finite_angles(angles)?;
    let n = pairs.len() as f64;
    let mut c = 0.0;
    let mut s = 0.0;
    for (cos_a, sin_a) in pairs {
        c += cos_a;
        s += sin_a;
    }
    Some(((c / n).hypot(s / n)).clamp(0.0, 1.0))
}

/// Circular standard deviation `√(-2 ln R)`.
///
/// Returns [`None`] when `R` is 0 (undefined) or there are no finite samples.
pub fn circular_sd(angles: &[f64]) -> Option<f64> {
    let r = mean_resultant_length(angles)?;
    if r <= 1e-12 {
        return None;
    }
    Some((-2.0 * r.ln()).sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn almost(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn wrap_pi_principal_interval() {
        assert!(almost(wrap_pi(0.0), 0.0));
        assert!(almost(wrap_pi(PI), PI));
        assert!(almost(wrap_pi(-PI), PI));
        assert!(almost(wrap_pi(3.0 * PI), PI));
        assert!(almost(wrap_pi(-1.5 * PI), 0.5 * PI));
    }

    #[test]
    fn angular_diff_across_cut() {
        let d = angular_diff(0.1, -0.1);
        assert!(almost(d, 0.2));
        let wrap = angular_diff(-3.0, 3.0);
        assert!(wrap.abs() < 0.3); // ~0.28, not ~6
        assert!(wrap > 0.0);
    }

    #[test]
    fn identical_angles_unit_resultant() {
        let a = [0.0, 0.0, 0.0];
        assert!(almost(circular_mean(&a).unwrap(), 0.0));
        assert!(almost(mean_resultant_length(&a).unwrap(), 1.0));
        assert!(circular_sd(&a).unwrap() < 1e-9);
    }

    #[test]
    fn opposite_angles_cancel() {
        let a = [0.0, PI];
        assert!(circular_mean(&a).is_none());
        assert!(mean_resultant_length(&a).unwrap() < 1e-12);
        assert!(circular_sd(&a).is_none());
    }

    #[test]
    fn empty_and_nonfinite_are_none() {
        assert!(circular_mean(&[]).is_none());
        assert!(mean_resultant_length(&[f64::NAN]).is_none());
        assert!(circular_sd(&[f64::INFINITY]).is_none());
    }
}
