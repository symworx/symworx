// core/src/statistics/errors.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

// ==========================================================
// Error measures
// ==========================================================
// Mean absolute error
// ----------------------------------------------------------
/// Mean absolute error between two vectors
///
/// # Arguments
/// * `actual` - Actual values
/// * `predicted` - Predicted values
///
/// # Returns
/// The mean absolute error as an f64. If the input slices
///   have different lengths, it returns NaN.
pub fn mae(actual: &[f64], predicted: &[f64]) -> f64 {
    if actual.len() != predicted.len() {
        return f64::NAN;
    }
    actual
        .iter()
        .zip(predicted.iter())
        .map(|(a, p)| (a - p).abs())
        .sum::<f64>()
        / actual.len() as f64
}


// ----------------------------------------------------------
// Mean squared error
// ----------------------------------------------------------
/// Mean squared error between two vectors
///
/// # Arguments
/// * `actual` - Actual values
/// * `predicted` - Predicted values
///
/// # Returns
/// The mean squared error as an f64. If the input slices
///   have different lengths, it returns NaN.
pub fn mse(actual: &[f64], predicted: &[f64]) -> f64 {
    if actual.len() != predicted.len() {
        return f64::NAN;
    }
    actual
        .iter()
        .zip(predicted.iter())
        .map(|(a, p)| (a - p).powi(2))
        .sum::<f64>()
        / actual.len() as f64
}


// ----------------------------------------------------------
// Root mean squared error
// ----------------------------------------------------------
/// Root mean squared error between two vectors
///
/// # Arguments
/// * `actual` - Actual values
/// * `predicted` - Predicted values
///
/// # Returns
/// The root mean squared error as an f64. If the input slices
///   have different lengths, it returns NaN.
pub fn rmse(actual: &[f64], predicted: &[f64]) -> f64
{
    mse(actual, predicted).sqrt()
}


// ==========================================================
// TESTS
// ==========================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mae() {
        let actual = [1.0, 2.0, 3.0];
        let predicted = [1.5, 2.5, 3.5];
        assert_eq!(mae(&actual, &predicted), 0.5);
    }

    #[test]
    fn test_mse() {
        let actual = [1.0, 2.0, 3.0];
        let predicted = [1.5, 2.5, 3.5];
        assert_eq!(mse(&actual, &predicted), 0.25);
    }

    #[test]
    fn test_rmse() {
        let actual = [1.0, 2.0, 3.0];
        let predicted = [1.5, 2.5, 3.5];
        assert_eq!(rmse(&actual, &predicted), 0.5);
    }
}
