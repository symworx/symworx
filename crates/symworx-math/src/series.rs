// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Series and sequential operations.
//!
//! This module is the **canonical home** in the SymWorx ecosystem for
//! low-level, allocation-minimal operations on ordered sequences
//! (time series, stride intervals, inter-beat intervals, RR intervals, etc.).
//!
//! ## Design Principles
//!
//! - **Signed by default**: The primary operation (`successive_differences`)
//!   returns signed deltas. This preserves directional information
//!   (e.g. speeding up vs slowing down).
//! - **Explicit absolute variant**: When only magnitude matters, use
//!   `successive_absolute_differences`.
//! - **Low-level & reusable**: These primitives are deliberately kept
//!   simple and allocation-minimal so they can be used by both
//!   modeling crates (`symworx-biosym`) and analysis crates
//!   (`symworx-stats`, `symworx-dynamics`).
//! - **Single source of truth**: Do not re-implement successive difference
//!   logic elsewhere. Depend on `symworx-math` (usually via `symworx-core`)
//!   instead.
//!
//! ## Relationship to Other Crates
//!
//! - `symworx-stats` re-exports these primitives and builds higher-level
//!   variability descriptors (`rmssd`, `sd_successive_differences`, etc.)
//!   on top of them.
//! - `symworx-biosym` uses them for gait and physiological interval
//!   calculations.
//! - `symworx-signal` (when it needs them) should depend on this module
//!   rather than duplicating logic.
//!
//! ## Adding New Operations
//!
//! New general-purpose sequence operations (e.g. cumulative sums,
//! forward/backward differences, simple rolling statistics) should be
//! added here rather than in `symworx-stats` or domain crates.

/// Computes the signed successive differences between consecutive elements.
///
/// This is the fundamental "delta" / first-order difference operation:
/// `data[i+1] - data[i]`.
///
/// This is the primary primitive in this module. Use this when directional
/// information matters (the default choice in most modeling and analysis code).
///
/// Returns an empty vector if `data.len() < 2`.
///
/// # Example
/// ```
/// use symworx_math::series::successive_differences;
///
/// let times = [0.0, 1.2, 2.5, 3.7];
/// let diffs = successive_differences(&times);
/// assert_eq!(diffs, vec![1.2, 1.3, 1.2]);
/// ```
pub fn successive_differences(data: &[f64]) -> Vec<f64> {
    if data.len() < 2 {
        return Vec::new();
    }
    data.windows(2).map(|w| w[1] - w[0]).collect()
}

/// Computes the absolute successive differences between consecutive elements.
///
/// Use this when only the magnitude of change matters (common in some
/// variability metrics). Most code should prefer the signed version
/// ([`successive_differences`]).
///
/// Returns an empty vector if `data.len() < 2`.
pub fn successive_absolute_differences(data: &[f64]) -> Vec<f64> {
    if data.len() < 2 {
        return Vec::new();
    }
    data.windows(2).map(|w| (w[1] - w[0]).abs()).collect()
}

// --- Optional zero-copy / iterator versions for advanced use ---

/// Returns an iterator over signed successive differences.
///
/// Allocation-free version of [`successive_differences`]. Useful for
/// one-pass processing or when you want to avoid an intermediate `Vec`.
pub fn successive_differences_iter(data: &[f64]) -> impl Iterator<Item = f64> + '_ {
    data.windows(2).map(|w| w[1] - w[0])
}

/// Returns an iterator over absolute successive differences.
///
/// Allocation-free version of [`successive_absolute_differences`].
pub fn successive_absolute_differences_iter(data: &[f64]) -> impl Iterator<Item = f64> + '_ {
    data.windows(2).map(|w| (w[1] - w[0]).abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_successive_differences_basic() {
        let data = [0.0, 1.2, 2.5, 3.7];
        let diffs = successive_differences(&data);
        assert_eq!(diffs, vec![1.2, 1.3, 1.2]);
    }

    #[test]
    fn test_successive_differences_signed() {
        // Explicitly test that we preserve sign (not absolute)
        let data = [10.0, 8.0, 12.0];
        let diffs = successive_differences(&data);
        assert_eq!(diffs, vec![-2.0, 4.0]);
    }

    #[test]
    fn test_successive_differences_too_short() {
        assert!(successive_differences(&[42.0]).is_empty());
        assert!(successive_differences(&[]).is_empty());
    }

    #[test]
    fn test_successive_absolute_differences() {
        let data = [10.0, 8.0, 12.0];
        let diffs = successive_absolute_differences(&data);
        assert_eq!(diffs, vec![2.0, 4.0]);
    }

    #[test]
    fn test_iter_versions() {
        let data = [0.0, 1.0, 3.0];
        let collected: Vec<_> = successive_differences_iter(&data).collect();
        assert_eq!(collected, vec![1.0, 2.0]);
    }
}
