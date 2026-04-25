// core/statistics/distance.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

// ==========================================================
// Distance measures
// ==========================================================
// ----------------------------------------------------------
// Euclidean distance
// ----------------------------------------------------------
/// Euclidean distance between two vectors
///
/// # Arguments
/// * `a` - First vector
/// * `b` - Second vector
///
/// # Returns
/// The Euclidean distance as an f64. If the input slices have different lengths, it
///   returns NaN.
pub fn euclidean(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

// ==========================================================
// TESTS
// ==========================================================

