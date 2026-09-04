// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

/// Linear interpolation of y(x) at new points x_new.
/// Assumes x is strictly increasing.
pub fn interp_linear(x: &[f64], y: &[f64], x_new: &[f64]) -> Vec<f64> {
    assert_eq!(x.len(), y.len());

    let n = x.len();
    let mut out = Vec::with_capacity(x_new.len());

    let mut j = 0;

    for &xn in x_new {
        while j + 1 < n && x[j + 1] < xn {
            j += 1;
        }

        if j + 1 == n {
            out.push(y[n - 1]);
            continue;
        }

        let x0 = x[j];
        let x1 = x[j + 1];
        let y0 = y[j];
        let y1 = y[j + 1];

        let t = (xn - x0) / (x1 - x0);
        out.push(y0 + t * (y1 - y0));
    }

    out
}

/// Linear interpolation of y(x) at new points x_new.
/// Alias for interp_linear, providing a familiar name to other
///   scientific computing libraries.
pub fn interp1(x: &[f64], y: &[f64], x_new: &[f64]) -> Vec<f64> {
    interp_linear(x, y, x_new)
}

/// Natural cubic spline interpolation of `y(x)` at `x_new`.
///
/// Assumes `x` is strictly increasing and `x.len() == y.len()`.
/// Values outside `[x[0], x[n-1]]` clamp to the end samples (no cubic
/// extrapolation). `n < 3` falls back to [`interp_linear`].
///
/// PCHIP / cubic Hermite is a later option if IBI overshoot is a problem.
pub fn interp_spline(x: &[f64], y: &[f64], x_new: &[f64]) -> Vec<f64> {
    assert_eq!(x.len(), y.len());
    let n = x.len();
    if n < 3 {
        return interp_linear(x, y, x_new);
    }

    let m = natural_spline_moments(x, y);
    let mut out = Vec::with_capacity(x_new.len());
    let mut j = 0;

    for &xn in x_new {
        if xn <= x[0] {
            out.push(y[0]);
            continue;
        }
        if xn >= x[n - 1] {
            out.push(y[n - 1]);
            continue;
        }
        while j + 1 < n && x[j + 1] < xn {
            j += 1;
        }
        out.push(spline_segment(x, y, &m, j, xn));
    }
    out
}

/// Cubic interpolation — v1 alias of [`interp_spline`] (natural cubic).
pub fn interp_cubic(x: &[f64], y: &[f64], x_new: &[f64]) -> Vec<f64> {
    interp_spline(x, y, x_new)
}

/// Second derivatives at knots; `m[0] = m[n-1] = 0` (natural ends).
fn natural_spline_moments(x: &[f64], y: &[f64]) -> Vec<f64> {
    let n = x.len();
    let mut m = vec![0.0; n];
    let k = n - 2;
    let mut sub = vec![0.0; k];
    let mut diag = vec![0.0; k];
    let mut sup = vec![0.0; k];
    let mut rhs = vec![0.0; k];

    for i in 1..n - 1 {
        let h0 = x[i] - x[i - 1];
        let h1 = x[i + 1] - x[i];
        let idx = i - 1;
        sub[idx] = h0;
        diag[idx] = 2.0 * (h0 + h1);
        sup[idx] = h1;
        rhs[idx] = 6.0 * ((y[i + 1] - y[i]) / h1 - (y[i] - y[i - 1]) / h0);
    }
    sub[0] = 0.0;
    sup[k - 1] = 0.0;

    let interior = thomas(&sub, &diag, &sup, &rhs);
    for (i, mi) in interior.into_iter().enumerate() {
        m[i + 1] = mi;
    }
    m
}

/// Thomas algorithm: `sub` below diagonal, `diag`, `sup` above; all length `k`.
fn thomas(sub: &[f64], diag: &[f64], sup: &[f64], rhs: &[f64]) -> Vec<f64> {
    let k = diag.len();
    let mut cp = vec![0.0; k];
    let mut dp = vec![0.0; k];
    cp[0] = sup[0] / diag[0];
    dp[0] = rhs[0] / diag[0];
    for i in 1..k {
        let denom = diag[i] - sub[i] * cp[i - 1];
        cp[i] = if i + 1 < k { sup[i] / denom } else { 0.0 };
        dp[i] = (rhs[i] - sub[i] * dp[i - 1]) / denom;
    }
    let mut z = vec![0.0; k];
    z[k - 1] = dp[k - 1];
    for i in (0..k - 1).rev() {
        z[i] = dp[i] - cp[i] * z[i + 1];
    }
    z
}

fn spline_segment(x: &[f64], y: &[f64], m: &[f64], j: usize, xn: f64) -> f64 {
    let h = x[j + 1] - x[j];
    let t = (xn - x[j]) / h;
    let a = y[j];
    let b = y[j + 1];
    let mj = m[j];
    let mj1 = m[j + 1];
    (1.0 - t) * a + t * b + ((1.0 - t) * ((1.0 - t) * (1.0 - t) - 1.0) * mj + t * (t * t - 1.0) * mj1) * (h * h) / 6.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spline_recovers_a_line() {
        let x = [0.0, 1.0, 2.0, 3.0];
        let y = [1.0, 3.0, 5.0, 7.0]; // y = 2x + 1
        let x_new = [0.5, 1.5, 2.5];
        let out = interp_spline(&x, &y, &x_new);
        for (yi, &xn) in out.iter().zip(&x_new) {
            assert!((yi - (2.0 * xn + 1.0)).abs() < 1e-10, "{yi} vs {}", 2.0 * xn + 1.0);
        }
    }

    #[test]
    fn spline_clamps_outside() {
        let x = [0.0, 1.0, 2.0, 3.0];
        let y = [0.0, 1.0, 0.0, 1.0];
        let out = interp_spline(&x, &y, &[-1.0, 4.0]);
        assert_eq!(out[0], 0.0);
        assert_eq!(out[1], 1.0);
    }

    #[test]
    fn cubic_aliases_spline() {
        let x = [0.0, 1.0, 2.0, 3.0];
        let y = [0.0, 1.0, 4.0, 9.0];
        let q = [0.5, 1.5];
        assert_eq!(interp_cubic(&x, &y, &q), interp_spline(&x, &y, &q));
    }

    #[test]
    fn two_points_falls_back_to_linear() {
        let x = [0.0, 2.0];
        let y = [0.0, 4.0];
        let out = interp_spline(&x, &y, &[1.0]);
        assert!((out[0] - 2.0).abs() < 1e-12);
    }

    #[test]
    fn spline_passes_through_knots() {
        let x = [0.0, 1.0, 2.0, 3.0, 4.0];
        let y = [0.0, 1.0, 0.0, 1.0, 0.0];
        let out = interp_spline(&x, &y, &x);
        for (a, b) in out.iter().zip(y.iter()) {
            assert!((a - b).abs() < 1e-10);
        }
    }
}
