// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Error and performance metrics for **predicted vs expected** values.
//!
//! Continuous (regression) scoring for predictive analytics and model
//! comparison (Kuhn & Johnson style). Residuals use the classical convention
//!
//! ```text
//! eᵢ = yᵢ − ŷᵢ   (actual − predicted)
//! ```
//!
//! so a **positive bias** (`mean(e)`) means the model under-predicts on average.
//!
//! Length mismatches return `NaN` (or an empty residual vector / zero-length
//! report) rather than panicking.

use std::fmt;

/// Computes the **Mean Absolute Error (MAE)** between two slices.
///
/// Returns `f64::NAN` if the slices have different lengths or are empty.
pub fn mae(actual: &[f64], predicted: &[f64]) -> f64 {
    if actual.len() != predicted.len() || actual.is_empty() {
        return f64::NAN;
    }

    actual
        .iter()
        .zip(predicted.iter())
        .map(|(a, p)| (a - p).abs())
        .sum::<f64>()
        / actual.len() as f64
}

/// Computes the **Mean Squared Error (MSE)** between two slices.
///
/// Returns `f64::NAN` if the slices have different lengths or are empty.
pub fn mse(actual: &[f64], predicted: &[f64]) -> f64 {
    if actual.len() != predicted.len() || actual.is_empty() {
        return f64::NAN;
    }

    actual
        .iter()
        .zip(predicted.iter())
        .map(|(a, p)| (a - p).powi(2))
        .sum::<f64>()
        / actual.len() as f64
}

/// Computes the **Root Mean Squared Error (RMSE)** between two slices.
///
/// Returns `f64::NAN` if the slices have different lengths or are empty.
pub fn rmse(actual: &[f64], predicted: &[f64]) -> f64 {
    mse(actual, predicted).sqrt()
}

/// Pointwise residuals `eᵢ = actualᵢ − predictedᵢ` (observed − fitted).
///
/// This is the **canonical** residual helper for model diagnostics. Feed the
/// result into [`crate::histogram`] / [`crate::kde_gaussian`] / Bland–Altman
/// plots, or summarize with [`mae`] / [`rmse`] / [`bias`] on
/// `(actual, predicted)` directly.
///
/// Returns an empty vector if lengths differ.
///
/// # Example
/// ```
/// use symworx_stats::{residuals, histogram_default, kde_gaussian_default};
///
/// let y = [1.0, 2.0, 3.0, 4.0];
/// let yhat = [1.1, 1.9, 3.2, 3.8];
/// let e = residuals(&y, &yhat);
/// assert_eq!(e.len(), 4);
/// let _hist = histogram_default(&e);
/// let _kde = kde_gaussian_default(&e);
/// ```
pub fn residuals(actual: &[f64], predicted: &[f64]) -> Vec<f64> {
    if actual.len() != predicted.len() {
        return Vec::new();
    }
    actual.iter().zip(predicted.iter()).map(|(a, p)| a - p).collect()
}

/// Alias for [`residuals`] — same convention `y − ŷ`.
#[inline]
pub fn residual_errors(actual: &[f64], predicted: &[f64]) -> Vec<f64> {
    residuals(actual, predicted)
}

/// Mean residual (bias): `mean(actual − predicted)`.
///
/// Positive ⇒ model under-predicts on average; negative ⇒ over-predicts.
/// Returns `f64::NAN` if lengths differ or inputs are empty.
pub fn bias(actual: &[f64], predicted: &[f64]) -> f64 {
    if actual.len() != predicted.len() || actual.is_empty() {
        return f64::NAN;
    }
    residuals(actual, predicted).iter().sum::<f64>() / actual.len() as f64
}

/// Maximum absolute residual `max |actual − predicted|`.
///
/// Returns `f64::NAN` if lengths differ or inputs are empty.
pub fn max_abs_error(actual: &[f64], predicted: &[f64]) -> f64 {
    if actual.len() != predicted.len() || actual.is_empty() {
        return f64::NAN;
    }
    actual
        .iter()
        .zip(predicted.iter())
        .map(|(a, p)| (a - p).abs())
        .fold(0.0_f64, f64::max)
}

/// Coefficient of determination R² = `1 − SS_res / SS_tot`.
///
/// * `SS_res = Σ (y − ŷ)²`
/// * `SS_tot = Σ (y − ȳ)²`
///
/// Returns `1.0` when predictions are perfect and total variance is zero
/// (constant series, perfect fit). Returns `f64::NAN` if lengths differ,
/// `n < 2`, or `SS_tot` is numerically zero while residuals are not.
pub fn r2(actual: &[f64], predicted: &[f64]) -> f64 {
    if actual.len() != predicted.len() || actual.len() < 2 {
        return f64::NAN;
    }
    let n = actual.len() as f64;
    let y_mean = actual.iter().sum::<f64>() / n;
    let mut ss_res = 0.0;
    let mut ss_tot = 0.0;
    for (a, p) in actual.iter().zip(predicted.iter()) {
        let e = a - p;
        ss_res += e * e;
        let d = a - y_mean;
        ss_tot += d * d;
    }
    if ss_tot < 1e-15 {
        // Constant actual series
        return if ss_res < 1e-15 { 1.0 } else { f64::NAN };
    }
    1.0 - ss_res / ss_tot
}

/// Bundle of continuous predicted-vs-expected metrics.
#[derive(Debug, Clone, PartialEq)]
pub struct RegressionReport {
    /// Number of paired samples (`0` if inputs were invalid).
    pub n: usize,
    /// Mean absolute error.
    pub mae: f64,
    /// Mean squared error.
    pub mse: f64,
    /// Root mean squared error.
    pub rmse: f64,
    /// Coefficient of determination.
    pub r2: f64,
    /// Mean residual `mean(y − ŷ)`.
    pub bias: f64,
    /// Maximum absolute residual.
    pub max_abs_error: f64,
}

impl RegressionReport {
    /// All-NaN report for invalid inputs.
    fn invalid() -> Self {
        Self {
            n: 0,
            mae: f64::NAN,
            mse: f64::NAN,
            rmse: f64::NAN,
            r2: f64::NAN,
            bias: f64::NAN,
            max_abs_error: f64::NAN,
        }
    }
}

impl fmt::Display for RegressionReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.n == 0 {
            return write!(f, "RegressionReport(invalid / empty)");
        }
        write!(
            f,
            "n={}  MAE={:.6}  RMSE={:.6}  R²={:.6}  bias={:.6}  max|e|={:.6}",
            self.n, self.mae, self.rmse, self.r2, self.bias, self.max_abs_error
        )
    }
}

/// Build a [`RegressionReport`] comparing `actual` (expected) to `predicted`.
///
/// Returns an invalid report (`n = 0`, NaN fields) if lengths differ or either
/// slice is empty.
pub fn regression_report(actual: &[f64], predicted: &[f64]) -> RegressionReport {
    if actual.len() != predicted.len() || actual.is_empty() {
        return RegressionReport::invalid();
    }
    RegressionReport {
        n: actual.len(),
        mae: mae(actual, predicted),
        mse: mse(actual, predicted),
        rmse: rmse(actual, predicted),
        r2: r2(actual, predicted),
        bias: bias(actual, predicted),
        max_abs_error: max_abs_error(actual, predicted),
    }
}

// TESTS
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics() {
        let actual = [1.0, 2.0, 3.0];
        let predicted = [1.5, 2.5, 3.5];

        assert_eq!(mae(&actual, &predicted), 0.5);
        assert_eq!(mse(&actual, &predicted), 0.25);
        assert_eq!(rmse(&actual, &predicted), 0.5);
    }

    #[test]
    fn test_mismatched_lengths() {
        let actual = [1.0, 2.0];
        let predicted = [1.0, 2.0, 3.0];

        assert!(mae(&actual, &predicted).is_nan());
        assert!(mse(&actual, &predicted).is_nan());
        assert!(rmse(&actual, &predicted).is_nan());
        assert!(r2(&actual, &predicted).is_nan());
        assert!(bias(&actual, &predicted).is_nan());
        assert!(max_abs_error(&actual, &predicted).is_nan());
        assert!(residuals(&actual, &predicted).is_empty());
        assert_eq!(regression_report(&actual, &predicted).n, 0);
    }

    #[test]
    fn test_perfect_fit() {
        let y = [1.0, 2.0, 3.0];
        let yhat = [1.0, 2.0, 3.0];
        assert!((r2(&y, &yhat) - 1.0).abs() < 1e-15);
        assert!((rmse(&y, &yhat)).abs() < 1e-15);
        assert!((bias(&y, &yhat)).abs() < 1e-15);
        assert!((max_abs_error(&y, &yhat)).abs() < 1e-15);
        let rep = regression_report(&y, &yhat);
        assert_eq!(rep.n, 3);
        assert!((rep.r2 - 1.0).abs() < 1e-15);
    }

    #[test]
    fn test_residuals_and_bias() {
        let y = [1.0, 2.0, 3.0];
        // ŷ always 0.5 low ⇒ residuals all +0.5, bias +0.5
        let yhat = [0.5, 1.5, 2.5];
        let e = residuals(&y, &yhat);
        assert_eq!(e, vec![0.5, 0.5, 0.5]);
        assert!((bias(&y, &yhat) - 0.5).abs() < 1e-15);
        assert!((max_abs_error(&y, &yhat) - 0.5).abs() < 1e-15);
    }

    #[test]
    fn test_r2_known_values() {
        // y = [1,2,3], ŷ = mean(y)=2 for all → R² = 0
        let y = [1.0, 2.0, 3.0];
        let yhat = [2.0, 2.0, 2.0];
        assert!((r2(&y, &yhat)).abs() < 1e-15);

        // ŷ = [1.5, 2.5, 3.5]: shifted by +0.5
        // SS_res = 3 * 0.25 = 0.75
        // ȳ = 2, SS_tot = 1+0+1 = 2
        // R² = 1 - 0.75/2 = 0.625
        let yhat2 = [1.5, 2.5, 3.5];
        assert!((r2(&y, &yhat2) - 0.625).abs() < 1e-12);
    }

    #[test]
    fn test_r2_constant_actual_perfect() {
        let y = [5.0, 5.0, 5.0];
        let yhat = [5.0, 5.0, 5.0];
        assert!((r2(&y, &yhat) - 1.0).abs() < 1e-15);
    }

    #[test]
    fn test_r2_too_short() {
        assert!(r2(&[1.0], &[1.0]).is_nan());
        assert!(r2(&[], &[]).is_nan());
    }

    #[test]
    fn test_regression_report_bundle() {
        let y = [1.0, 2.0, 3.0];
        let yhat = [1.5, 2.5, 3.5];
        let rep = regression_report(&y, &yhat);
        assert_eq!(rep.n, 3);
        assert!((rep.mae - 0.5).abs() < 1e-15);
        assert!((rep.mse - 0.25).abs() < 1e-15);
        assert!((rep.rmse - 0.5).abs() < 1e-15);
        assert!((rep.r2 - 0.625).abs() < 1e-12);
        assert!((rep.bias + 0.5).abs() < 1e-15); // y − ŷ = −0.5
        assert!((rep.max_abs_error - 0.5).abs() < 1e-15);

        let s = format!("{rep}");
        assert!(s.contains("R²"));
        assert!(s.contains("n=3"));
    }

    #[test]
    fn test_empty_inputs() {
        let empty: [f64; 0] = [];
        assert!(mae(&empty, &empty).is_nan());
        assert_eq!(regression_report(&empty, &empty).n, 0);
    }
}
