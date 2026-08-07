// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Design-matrix helpers for longitudinal / multilevel models.
//!
//! Pure array construction (no fitting). Use with [`super::RandomTerm`] and
//! fixed-effect design matrices passed to [`super::lmer`].
//!
//! Conventions match the rest of `symworx-stats`: fixed designs for
//! [`super::lmer`] / OLS omit the intercept column when
//! [`super::LmerConfig::fit_intercept`] is `true`. Random-effect designs
//! (`z_cols`) usually **include** an intercept column of ones when a random
//! intercept is desired.

use ndarray::{
    Array1,
    Array2,
};

use super::types::MixedError;

/// Sample mean of `t` (empty → error).
pub fn mean_time(t: &Array1<f64>) -> Result<f64, MixedError> {
    if t.is_empty() {
        return Err(MixedError::EmptyData);
    }
    Ok(t.mean().unwrap_or(0.0))
}

/// Center time at its sample mean: `t' = t − mean(t)`.
///
/// Returns `(centered, mean)`.
pub fn center_time(t: &Array1<f64>) -> Result<(Array1<f64>, f64), MixedError> {
    let m = mean_time(t)?;
    Ok((t - m, m))
}

/// Center time at a chosen origin: `t' = t − origin`.
pub fn center_time_at(t: &Array1<f64>, origin: f64) -> Array1<f64> {
    t - origin
}

/// Polynomial powers of time without a constant column: `[t, t², …, t^d]`.
///
/// Degree `0` yields an empty feature matrix with `t.len()` rows (same idea as
/// [`crate::polynomial_design`]). Degree `1` is a single time column.
pub fn time_powers(t: &Array1<f64>, degree: usize) -> Array2<f64> {
    let n = t.len();
    if degree == 0 {
        return Array2::zeros((n, 0));
    }
    let mut design = Array2::<f64>::zeros((n, degree));
    for i in 0..n {
        let mut pow = t[i];
        for d in 0..degree {
            design[[i, d]] = pow;
            pow *= t[i];
        }
    }
    design
}

/// Random-effect design for intercept + linear slope: columns `[1, t]`.
///
/// Shape `n × 2`. Typical input to [`super::RandomTerm::z_cols`] for a linear
/// growth random structure (pair with [`super::CovStructure::Unstructured`]).
pub fn z_intercept_slope(t: &Array1<f64>) -> Array2<f64> {
    let n = t.len();
    let mut z = Array2::<f64>::ones((n, 2));
    for i in 0..n {
        z[[i, 1]] = t[i];
    }
    z
}

/// Random-effect design: intercept only (`n × 1` ones).
pub fn z_intercept(n: usize) -> Array2<f64> {
    Array2::ones((n, 1))
}

/// Hinge (truncated power) columns: `max(0, t − k)` for each knot `k`.
///
/// Used for piecewise-linear change in slope after each knot. Does not include
/// a baseline intercept or pre-knot slope; combine with [`time_powers`] /
/// [`z_intercept_slope`] as needed.
///
/// Knots should be strictly increasing for interpretability (not enforced).
pub fn piecewise_hinges(t: &Array1<f64>, knots: &[f64]) -> Array2<f64> {
    let n = t.len();
    let k = knots.len();
    let mut out = Array2::<f64>::zeros((n, k));
    for (j, &knot) in knots.iter().enumerate() {
        for i in 0..n {
            let d = t[i] - knot;
            out[[i, j]] = if d > 0.0 { d } else { 0.0 };
        }
    }
    out
}

/// Fixed-effect piecewise linear basis without intercept:
/// `[t, max(0,t−k₁), …, max(0,t−kₘ)]`.
///
/// Pair with `fit_intercept = true` so the model has a global intercept plus
/// a baseline slope and slope changes at each knot.
pub fn fixed_piecewise_linear(t: &Array1<f64>, knots: &[f64]) -> Array2<f64> {
    let base = time_powers(t, 1);
    if knots.is_empty() {
        return base;
    }
    let hinges = piecewise_hinges(t, knots);
    hstack(&[&base, &hinges])
}

/// Column-bind designs with the same number of rows.
///
/// # Errors
/// [`MixedError::EmptyData`] if `parts` is empty;
/// [`MixedError::LengthMismatch`] if row counts differ.
pub fn hstack(parts: &[&Array2<f64>]) -> Array2<f64> {
    assert!(!parts.is_empty(), "hstack requires at least one block");
    let n = parts[0].nrows();
    let cols: usize = parts.iter().map(|p| p.ncols()).sum();
    let mut out = Array2::<f64>::zeros((n, cols));
    let mut c0 = 0;
    for p in parts {
        assert_eq!(p.nrows(), n, "hstack row mismatch");
        let c1 = c0 + p.ncols();
        out.slice_mut(ndarray::s![.., c0..c1]).assign(p);
        c0 = c1;
    }
    out
}

/// Fallible [`hstack`] with [`MixedError`] instead of assert.
pub fn try_hstack(parts: &[&Array2<f64>]) -> Result<Array2<f64>, MixedError> {
    if parts.is_empty() {
        return Err(MixedError::EmptyData);
    }
    let n = parts[0].nrows();
    for (i, p) in parts.iter().enumerate() {
        if p.nrows() != n {
            return Err(MixedError::LengthMismatch {
                what: format!("hstack block {i} rows"),
                expected: n,
                got: p.nrows(),
            });
        }
    }
    Ok(hstack(parts))
}

/// Build `Z = [1, t]` after centering time at `origin` (or sample mean if `None`).
///
/// Returns `(z, time_centered, origin_used)`.
pub fn z_intercept_slope_centered(
    t: &Array1<f64>,
    origin: Option<f64>,
) -> Result<(Array2<f64>, Array1<f64>, f64), MixedError> {
    let (tc, origin_used) = match origin {
        Some(o) => (center_time_at(t, o), o),
        None => center_time(t)?,
    };
    Ok((z_intercept_slope(&tc), tc, origin_used))
}

#[cfg(all(test, feature = "linalg"))]
mod tests {
    use ndarray::array;

    use super::*;

    #[test]
    fn center_time_zero_mean() {
        let t = array![1.0, 2.0, 3.0, 4.0];
        let (c, m) = center_time(&t).unwrap();
        assert!((m - 2.5).abs() < 1e-12);
        assert!((c.mean().unwrap()).abs() < 1e-12);
    }

    #[test]
    fn time_powers_degree() {
        let t = array![2.0, 3.0];
        let p = time_powers(&t, 2);
        assert_eq!(p.shape(), &[2, 2]);
        assert!((p[[0, 0]] - 2.0).abs() < 1e-12);
        assert!((p[[0, 1]] - 4.0).abs() < 1e-12);
        assert!((p[[1, 0]] - 3.0).abs() < 1e-12);
        assert!((p[[1, 1]] - 9.0).abs() < 1e-12);
    }

    #[test]
    fn z_intercept_slope_shape() {
        let t = array![0.0, 1.0, 2.0];
        let z = z_intercept_slope(&t);
        assert_eq!(z.shape(), &[3, 2]);
        assert!((z[[1, 0]] - 1.0).abs() < 1e-12);
        assert!((z[[1, 1]] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn piecewise_hinges_and_fixed() {
        let t = array![0.0, 1.0, 2.0, 3.0, 4.0];
        let h = piecewise_hinges(&t, &[2.0]);
        assert!((h[[1, 0]] - 0.0).abs() < 1e-12);
        assert!((h[[3, 0]] - 1.0).abs() < 1e-12);
        let fx = fixed_piecewise_linear(&t, &[2.0]);
        assert_eq!(fx.ncols(), 2); // t and hinge
        assert!((fx[[4, 0]] - 4.0).abs() < 1e-12);
        assert!((fx[[4, 1]] - 2.0).abs() < 1e-12);
    }

    #[test]
    fn try_hstack_mismatch() {
        let a = Array2::<f64>::zeros((2, 1));
        let b = Array2::<f64>::zeros((3, 1));
        let err = try_hstack(&[&a, &b]).unwrap_err();
        assert!(matches!(err, MixedError::LengthMismatch { .. }));
    }
}
