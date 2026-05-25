// Copyright (C) 2026 cSYMd, All rights reserved.

//! Error and performance metrics
//!
//! Common regression and signal evaluation metrics.

/// Computes the **Mean Absolute Error (MAE)** between two slices.
///
/// Returns `f64::NAN` if the slices have different lengths.
pub fn mae(actual: &[f64], predicted: &[f64]) -> f64 {
    if actual.len() != predicted.len() {
        return f64::NAN;
    }

    actual.iter()
        .zip(predicted.iter())
        .map(|(a, p)| (a - p).abs())
        .sum::<f64>() / actual.len() as f64
}

/// Computes the **Mean Squared Error (MSE)** between two slices.
///
/// Returns `f64::NAN` if the slices have different lengths.
pub fn mse(actual: &[f64], predicted: &[f64]) -> f64 {
    if actual.len() != predicted.len() {
        return f64::NAN;
    }

    actual.iter()
        .zip(predicted.iter())
        .map(|(a, p)| (a - p).powi(2))
        .sum::<f64>() / actual.len() as f64
}

/// Computes the **Root Mean Squared Error (RMSE)** between two slices.
///
/// Returns `f64::NAN` if the slices have different lengths.
pub fn rmse(actual: &[f64], predicted: &[f64]) -> f64 {
    mse(actual, predicted).sqrt()
}


// --- TESTS ---
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
    }
}
