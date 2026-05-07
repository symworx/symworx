// symworx-core/src/math/integration.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

/// Cumulative trapezoidal integration
///
/// # Arguments
///
/// # Returns
///
#[inline]
pub fn cumtrapz(y: &[f64], dx: f64) -> Vec<f64> {
    let n = y.len();
    if n == 0 {
        return vec![];
    }

    let mut out = Vec::with_capacity(n);
    out.push(0.0);

    let mut acc = 0.0;
    for i in 1..n {
        acc += 0.5 * (y[i - 1] + y[i]) * dx;
        out.push(acc);
    }

    out
}

/// Single trapezoidal integral over the whole array.
///
/// # Arguments
///
/// # Returns
///
#[inline]
pub fn trapz(y: &[f64], dx: f64) -> f64 {
    let n = y.len();
    if n < 2 {
        return 0.0;
    }

    let mut acc = 0.0;
    for i in 1..n {
        acc += 0.5 * (y[i - 1] + y[i]) * dx;
    }
    acc
}
