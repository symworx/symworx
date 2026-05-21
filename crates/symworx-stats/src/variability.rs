// symworx/crates/symworx-stats/src/variability.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

// ==========================================================
// Variability measures
// ==========================================================
// Intervals
// ----------------------------------------------------------
/// Compuate the interbeat intervals of a vector
///
/// # Arguments
/// * `data` - Input vector
///
/// # Returns
/// A vector of interbeat intervals. If the input slice is
///   empty, it returns an empty
pub fn intervals(data: &[f64]) -> Vec<f64> {
    if data.len() < 2 {
        return Vec::new();
    }
    data.windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .collect()
}

// ----------------------------------------------------------
// Iterbeat interval
// ----------------------------------------------------------
/// Iterbeat interval of a vector
///
/// # Arguments
/// * `data` - Input vector
///
/// # Returns
/// The mean iterbeat interval as an f64. If the input slice is
/// empty, it returns NaN.
pub fn ibi(data: &[f64]) -> f64 {
    if data.is_empty() {
        return f64::NAN;
    }

    // Differences between consecutive beats
    let sum: f64 = data
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .sum();

    sum / (data.len() - 1) as f64
}

// ----------------------------------------------------------
// Root mean square of successive differences
// ----------------------------------------------------------
/// Root mean square of successive differences of a vector
///
/// # Arguments
/// * `data` - Input vector
///
/// # Returns
/// The root mean square of successive differences as an f64.
///   If the input slice has fewer than 3 elements, it returns NaN.
pub fn rmssd(data: &[f64]) -> f64 {
    if data.len() < 3 {
        return f64::NAN;
    }

    let sum: f64 = data
        .windows(3)
        .map(|w| (w[2] - w[0]).abs().powi(2))
        .sum();

    (sum / (data.len() - 2) as f64).sqrt()
}

// ----------------------------------------------------------
// Standard deviation of interbeat intervals
// ----------------------------------------------------------
/// Standard deviation of interbeat intervals of a vector
///
/// # Arguments
/// * `data` - Input vector
///
/// # Returns
/// The standard deviation of interbeat intervals as an f64.
///   If the input slice has fewer than 2 elements, it returns NaN.
pub fn sdnn(data: &[f64]) -> f64 {
    if data.len() < 2 {
        return f64::NAN;
    }

    // let mean = ibi(data);
    let sum: f64 = data
        .windows(2)
        .map(|w| (w[1] - w[0]).abs().powi(2))
        .sum();

    (sum / (data.len() - 1) as f64).sqrt()
}


// ==========================================================
// TESTS
// ==========================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intervals() {
        let data = [1.0, 2.0, 4.0];
        let expected = vec![1.0, 2.0];
        assert_eq!(intervals(&data), expected);
    }

    #[test]
    fn test_ibi() {
        let data = [1.0, 2.0, 4.0];
        let expected = 1.5;
        assert_eq!(ibi(&data), expected);
    }

    #[test]
    fn test_rmssd() {
        let data = [1.0, 2.0, 4.0];
        let expected = 1.0;
        assert_eq!(rmssd(&data), expected);
    }

    #[test]
    fn test_sdnn() {
        let data = [1.0, 2.0, 4.0];
        let expected = 0.816496580927726;
        assert_eq!(sdnn(&data), expected);
    }
}
