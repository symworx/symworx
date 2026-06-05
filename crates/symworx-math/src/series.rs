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
/// let times = [0.0, 1.0, 3.0, 6.0];
/// let diffs = successive_differences(&times);
/// assert_eq!(diffs, vec![1.0, 2.0, 3.0]);
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

// ============================================================
// Rolling window statistics (canonical home per AGENTS.md)
// These power ACWR, EWMA, and load monitoring calculations
// in symworx-loadsym and other consumers.
// ============================================================

/// Computes a simple rolling (moving) mean over a sliding window.
///
/// Returns a Vec of the same length as `data`.
/// The first `window - 1` entries are `f64::NAN` (insufficient history).
/// Window size 0 or 1 returns NaNs for all (or original for w=1).
///
/// This is the building block for acute/chronic load windows (e.g. 7d, 28d).
pub fn rolling_mean(data: &[f64], window: usize) -> Vec<f64> {
    if window == 0 {
        return vec![f64::NAN; data.len()];
    }
    if window == 1 {
        return data.to_vec();
    }
    let n = data.len();
    if n == 0 {
        return vec![];
    }
    let mut out = vec![f64::NAN; n];
    if window > n {
        return out;
    }
    let mut sum: f64 = data[..window].iter().sum();
    out[window - 1] = sum / window as f64;
    for i in window..n {
        sum += data[i] - data[i - window];
        out[i] = sum / window as f64;
    }
    out
}

/// Computes rolling population standard deviation over a sliding window.
///
/// Same NaN prefix semantics as [`rolling_mean`].
/// Useful for monotony calculations (sd of daily loads).
pub fn rolling_std(data: &[f64], window: usize) -> Vec<f64> {
    if window <= 1 {
        return vec![f64::NAN; data.len()];
    }
    let n = data.len();
    if n == 0 {
        return vec![];
    }
    let mut out = vec![f64::NAN; n];
    if window > n {
        return out;
    }
    // Use two-pass per window for simplicity + correctness (small windows)
    for i in (window - 1)..n {
        let start = i + 1 - window;
        let slice = &data[start..=i];
        let mu = slice.iter().sum::<f64>() / window as f64;
        let var = slice.iter().map(|&x| (x - mu).powi(2)).sum::<f64>() / window as f64;
        out[i] = var.sqrt();
    }
    out
}

/// Exponentially Weighted Moving Average (EWMA).
///
/// `span` controls the decay (common in sports science: span=7 or 28).
/// alpha = 2 / (span + 1). Matches pandas `ewm(span=...)` convention.
///
/// Returns same-length Vec. First value = data[0] (no prior).
/// NaN inputs propagate naturally.
pub fn ewma(data: &[f64], span: usize) -> Vec<f64> {
    if data.is_empty() {
        return vec![];
    }
    if span == 0 {
        return vec![f64::NAN; data.len()];
    }
    let alpha = 2.0 / (span as f64 + 1.0);
    let mut out = Vec::with_capacity(data.len());
    let mut prev = data[0];
    out.push(prev);
    for &x in &data[1..] {
        let next = alpha * x + (1.0 - alpha) * prev;
        out.push(next);
        prev = next;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_successive_differences_basic() {
        let data = [0.0, 1.2, 2.5, 3.7];
        let diffs = successive_differences(&data);
        // Use tolerance because 1.2/2.5/3.7 are not exactly representable in f64
        assert_eq!(diffs.len(), 3);
        assert!((diffs[0] - 1.2).abs() < 1e-12);
        assert!((diffs[1] - 1.3).abs() < 1e-12);
        assert!((diffs[2] - 1.2).abs() < 1e-12);
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

    #[test]
    fn test_rolling_mean_basic() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        let rm = rolling_mean(&data, 3);
        assert!(rm[0].is_nan() && rm[1].is_nan());
        assert!((rm[2] - 2.0).abs() < 1e-12);
        assert!((rm[3] - 3.0).abs() < 1e-12);
        assert!((rm[4] - 4.0).abs() < 1e-12);
    }

    #[test]
    fn test_rolling_mean_window_too_large() {
        let data = [10.0, 20.0];
        let rm = rolling_mean(&data, 5);
        assert!(rm.iter().all(|v| v.is_nan()));
    }

    #[test]
    fn test_rolling_std() {
        let data = [2.0, 2.0, 2.0, 2.0];
        let rs = rolling_std(&data, 2);
        assert!(rs[0].is_nan());
        assert!((rs[1] - 0.0).abs() < 1e-12);
    }

    #[test]
    fn test_ewma_matches_alpha() {
        let data = [1.0, 2.0, 3.0];
        // span=1 => alpha=1.0 (follows exactly)
        let e = ewma(&data, 1);
        assert_eq!(e, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_ewma_decay() {
        let data = [10.0, 10.0, 10.0, 10.0];
        let e = ewma(&data, 3); // alpha ≈ 0.5
        assert!((e[3] - 10.0).abs() < 1e-10);
    }
}
