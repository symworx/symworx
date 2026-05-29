// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

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

/// Placeholder for cubic interpolation.
/// Currently unimplemented — returns a clone of `y` and logs a warning.
pub fn interp_cubic(_x: &[f64], y: &[f64], _x_new: &[f64]) -> Vec<f64> {
    eprintln!("Warning: cubic interpolation is not implemented yet");
    y.to_vec()
}

/// Placeholder for spline interpolation.
/// Currently unimplemented — returns a clone of `y` and logs a warning.
pub fn interp_spline(_x: &[f64], y: &[f64], _x_new: &[f64]) -> Vec<f64> {
    eprintln!("Warning: spline interpolation is not implemented yet");
    y.to_vec()
}
