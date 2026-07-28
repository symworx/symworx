// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Distance metrics between vectors.
//!
//! Common mathematical distances used in signal processing,
//! statistics, and machine learning.

/// Euclidean distance (L2 norm) between two vectors.
pub fn euclidean(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() {
        return f64::NAN;
    }
    if a.is_empty() {
        return 0.0;
    }

    a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum::<f64>().sqrt()
}

/// Manhattan distance (L1 norm) between two vectors.
pub fn manhattan(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() {
        return f64::NAN;
    }
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum()
}

/// Cosine distance between two vectors (1 - cosine similarity).
///
/// Returns `NaN` if either vector has zero magnitude.
pub fn cosine_distance(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return f64::NAN;
    }

    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|&x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|&x| x * x).sum::<f64>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return f64::NAN;
    }

    1.0 - (dot / (norm_a * norm_b))
}

/// Chebyshev distance (L∞ norm) — maximum absolute difference.
/// Useful for worst-case deviation.
pub fn chebyshev(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() {
        return f64::NAN;
    }
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).fold(0.0, f64::max)
}

// ——— Tests ———

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_euclidean() {
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 5.0, 6.0];
        assert!((euclidean(&a, &b) - 5.1961524227).abs() < 1e-8);
    }

    #[test]
    fn test_manhattan() {
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 5.0, 6.0];
        assert_eq!(manhattan(&a, &b), 9.0);
    }

    #[test]
    fn test_cosine_distance() {
        let a = [1.0, 2.0, 3.0];
        let b = [2.0, 4.0, 6.0];
        assert_eq!(cosine_distance(&a, &b), 0.0);
    }

    #[test]
    fn test_chebyshev() {
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 5.0, 6.0];
        assert_eq!(chebyshev(&a, &b), 3.0);
    }

    #[test]
    fn test_mismatched_lengths() {
        let a = [1.0, 2.0];
        let b = [1.0, 2.0, 3.0];
        assert!(euclidean(&a, &b).is_nan());
        assert!(manhattan(&a, &b).is_nan());
        assert!(chebyshev(&a, &b).is_nan());
    }
}
