// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Series and sequential operations.
//!
//! **Canonical home** for low-level, allocation-minimal ops on ordered sequences
//! (time series, stride/IBI/RR intervals, etc.).
//!
//! - **Signed by default**: [`successive_differences`] returns signed deltas
//!   (direction preserved). Use [`successive_absolute_differences`] for magnitude only.
//! - Do not re-implement successive differences elsewhere — depend on this module
//!   (usually via `symworx-core`). Higher-level variability lives in `symworx-stats`;
//!   new general sequence ops (windows, cumulative sums, …) should be implemented here.

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

// -----------------------------------------------------------------------------
// Windowing and segmentation primitives
// -----------------------------------------------------------------------------

/// Returns an iterator over contiguous sliding windows of the data.
///
/// Each yielded slice has exactly `window` elements. Windows advance by `step`.
/// If `step == 0` or `window == 0`, or the data is too short, the iterator yields nothing.
///
/// This is allocation-free and reuses the original slice memory (zero-copy views).
/// Useful building block for windowed feature extraction (e.g. RMSSD, sample entropy
/// over 30 s / 60 s segments of a resampled tachogram).
///
/// # Example
/// ```
/// use symworx_math::series::sliding_windows;
/// let data = [1.0, 2.0, 3.0, 4.0, 5.0];
/// let wins: Vec<_> = sliding_windows(&data, 3, 2).collect();
/// assert_eq!(wins, vec![&data[0..3], &data[2..5]]);
/// ```
pub fn sliding_windows(data: &[f64], window: usize, step: usize) -> impl Iterator<Item = &[f64]> + '_ {
    (0..)
        .map(move |i| {
            let start = i * step;
            let end = start + window;
            if end <= data.len() && window > 0 && step > 0 {
                Some(&data[start..end])
            } else {
                None
            }
        })
        .take_while(Option::is_some)
        .map(Option::unwrap)
}

/// Applies a function to each sliding window and collects the results.
///
/// `window` is the window length in samples. `step` controls overlap (step=1 is maximal overlap).
/// The returned vector has length equal to the number of valid windows.
/// Early windows that cannot be formed return no entry (length is data-dependent, not padded).
///
/// This generalizes the existing `rolling_mean` / `rolling_std` logic and is the
/// recommended way to compute windowed statistics or complexity measures (e.g. per-window
/// sample entropy on HRV or load series) without duplicating window iteration.
///
/// The closure receives a slice of exactly `window` elements.
pub fn rolling_apply<F, R>(data: &[f64], window: usize, step: usize, mut f: F) -> Vec<R>
where
    F: FnMut(&[f64]) -> R,
{
    if window == 0 || step == 0 || data.len() < window {
        return vec![];
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i + window <= data.len() {
        out.push(f(&data[i..i + window]));
        i += step;
    }
    out
}

/// Computes index ranges for time-based (duration) windows on a (possibly irregular)
/// time vector.
///
/// Returns `Vec<(start_idx, end_idx)>` (half-open, end exclusive) such that for each
/// range the times satisfy `times[end-1] - times[start] < window_sec` (approximately
/// covering `window_sec` of data) and successive windows are separated by approximately
/// `step_sec`.
///
/// This is the primitive needed to produce aligned 30 s / 60 s feature windows from
/// RR event times (or resampled tachogram times) and to pair them with external
/// epoch data such as delta power from PSG.
///
/// Empty or non-monotonic input yields empty result. `step_sec <= 0` or `window_sec <= 0`
/// also yields empty.
pub fn time_windows(times: &[f64], window_sec: f64, step_sec: f64) -> Vec<(usize, usize)> {
    if times.len() < 2 || window_sec <= 0.0 || step_sec <= 0.0 {
        return vec![];
    }

    let mut segments = Vec::new();
    let mut i = 0usize;

    while i < times.len() {
        // find the farthest j such that times[j] - times[i] < window_sec
        let t0 = times[i];
        let mut j = i + 1;
        while j < times.len() && (times[j] - t0) < window_sec {
            j += 1;
        }
        if j > i + 1 {
            // at least two points to form a meaningful window
            segments.push((i, j));
        }

        // advance i to the first index whose time is >= t0 + step_sec
        let t_target = t0 + step_sec;
        while i < times.len() && times[i] < t_target {
            i += 1;
        }
        if i >= times.len() {
            break;
        }
    }

    segments
}

// TESTS

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

    // --- New windowing primitives tests ---

    #[test]
    fn test_sliding_windows_basic() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        let wins: Vec<_> = sliding_windows(&data, 3, 2).collect();
        assert_eq!(wins.len(), 2);
        assert_eq!(wins[0], &data[0..3]);
        assert_eq!(wins[1], &data[2..5]);
    }

    #[test]
    fn test_sliding_windows_no_overlap() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let wins: Vec<_> = sliding_windows(&data, 2, 2).collect();
        assert_eq!(wins.len(), 3);
    }

    #[test]
    fn test_sliding_windows_too_short_or_zero() {
        let data = [1.0, 2.0];
        assert_eq!(sliding_windows(&data, 3, 1).count(), 0);
        assert_eq!(sliding_windows(&data, 0, 1).count(), 0);
        assert_eq!(sliding_windows(&data, 2, 0).count(), 0);
    }

    #[test]
    fn test_rolling_apply_basic() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        let means = rolling_apply(&data, 3, 2, |w| w.iter().sum::<f64>() / w.len() as f64);
        assert_eq!(means.len(), 2);
        assert!((means[0] - 2.0).abs() < 1e-12);
        assert!((means[1] - 4.0).abs() < 1e-12);
    }

    #[test]
    fn test_rolling_apply_matches_rolling_mean() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let from_apply = rolling_apply(&data, 3, 1, |w| w.iter().sum::<f64>() / 3.0);
        let from_rolling = rolling_mean(&data, 3);
        let valid_from_rolling: Vec<_> = from_rolling.iter().skip(2).copied().collect();
        assert_eq!(from_apply.len(), valid_from_rolling.len());
        for (a, b) in from_apply.iter().zip(valid_from_rolling.iter()) {
            assert!((a - b).abs() < 1e-12);
        }
    }

    #[test]
    fn test_time_windows_basic() {
        let times: Vec<f64> = (0..5).map(|i| i as f64).collect();
        let segs = time_windows(&times, 2.5, 2.0);
        assert!(!segs.is_empty());
        assert!(segs[0].0 == 0 && segs[0].1 >= 3);
    }

    #[test]
    fn test_time_windows_edge_cases() {
        let times = vec![0.0, 0.5, 1.0];
        assert!(time_windows(&times, 0.0, 1.0).is_empty());
        assert!(time_windows(&times, 10.0, 0.0).is_empty());
        assert!(time_windows(&[0.0], 1.0, 1.0).is_empty());
        let empty: Vec<f64> = vec![];
        assert!(time_windows(&empty, 1.0, 1.0).is_empty());
    }
}
