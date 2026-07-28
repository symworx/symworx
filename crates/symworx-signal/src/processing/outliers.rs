// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Outlier detection and robust interpolation ("dynamics interpolation").
//!
//! These utilities are intended for cleaning irregular or noisy series such as
//! RR/IBI intervals before windowed analysis (RMSSD, sample entropy, etc.) or
//! before feeding feature streams to a state estimator (e.g. Kalman for slow
//! drifting autonomic capacity).
//!
//! The implementation reuses:
//! - `symworx_stats::basic::{median, mad}` for robust local statistics
//! - existing linear interpolation from this crate for replacement
//!
//! "dynamics interpolation" is supported via the `LocalMedian` / `LocalMean`
//! and `LinearInterp` strategies on detected outliers (common for RR artifact
//! correction in the HRV / nonlinear dynamics literature).

use symworx_stats::basic::{
    mad,
    median,
};

use crate::processing::interpolation::interp_linear;

/// Criterion used to flag a sample as an outlier.
///
/// All criteria are applied point-wise with local context where relevant.
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum OutlierCriterion {
    /// Local Median Absolute Deviation rule: flag if |x - local_med| > k * local_mad.
    /// `half_window` defines the symmetric local neighborhood size (total 2*half+1).
    /// Typical k values: 3.0 (aggressive) or 5.0+ (conservative) for RR data.
    LocalMAD { half_window: usize, k: f64 },

    /// Flag if the absolute relative change from the previous sample exceeds the threshold.
    /// E.g. `PercentChange(0.20)` flags >20 % jumps (common ectopic rule of thumb).
    PercentChange(f64),

    /// Flag if absolute deviation from the previous sample exceeds the threshold (in same units).
    Absolute(f64),
}

/// Strategy for replacing flagged outlier values.
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum FillStrategy {
    /// Replace with the median of a local symmetric window around the outlier.
    LocalMedian { half_window: usize },
    /// Replace with the mean of a local symmetric window around the outlier.
    LocalMean { half_window: usize },
    /// Linearly interpolate from the nearest valid (non-outlier) neighbors.
    /// Falls back to forward/backward fill at the ends.
    LinearInterp,
}

/// Detect indices of outliers according to the given criterion.
///
/// For `LocalMAD` the local window is clamped at the boundaries.
/// Returns indices in ascending order (may contain duplicates only on degenerate input).
pub fn detect_outliers(data: &[f64], crit: OutlierCriterion) -> Vec<usize> {
    let n = data.len();
    if n == 0 {
        return vec![];
    }

    match crit {
        OutlierCriterion::LocalMAD { half_window, k } => {
            if half_window == 0 || k <= 0.0 {
                return vec![];
            }
            let mut bad = Vec::new();
            for i in 0..n {
                let start = i.saturating_sub(half_window);
                let end = (i + half_window + 1).min(n);
                let local = &data[start..end];
                let med = median(local);
                if med.is_nan() {
                    continue;
                }
                let m = mad(local, med);
                if m > 0.0 && (data[i] - med).abs() > k * m {
                    bad.push(i);
                }
            }
            bad
        }
        OutlierCriterion::PercentChange(threshold) => {
            if threshold <= 0.0 || n < 2 {
                return vec![];
            }
            let mut bad = Vec::new();
            for i in 1..n {
                let prev = data[i - 1];
                if prev == 0.0 {
                    // avoid div0; treat large absolute jump as suspicious instead
                    if data[i].abs() > threshold * 100.0 {
                        // very rough fallback
                        bad.push(i);
                    }
                    continue;
                }
                let pct = (data[i] - prev).abs() / prev.abs();
                if pct > threshold {
                    bad.push(i);
                }
            }
            bad
        }
        OutlierCriterion::Absolute(threshold) => {
            if threshold <= 0.0 || n < 2 {
                return vec![];
            }
            let mut bad = Vec::new();
            for i in 1..n {
                if (data[i] - data[i - 1]).abs() > threshold {
                    bad.push(i);
                }
            }
            bad
        }
    }
}

/// Replace the values at the given `outlier_indices` (must be sorted or will be handled)
/// using the chosen fill strategy. Returns a new vector of same length as `data`.
///
/// Non-outlier values are copied unchanged. Outlier indices outside range are ignored.
pub fn interpolate_outliers(data: &[f64], outlier_indices: &[usize], strat: FillStrategy) -> Vec<f64> {
    if data.is_empty() {
        return vec![];
    }
    let mut out = data.to_vec();
    if outlier_indices.is_empty() {
        return out;
    }

    // Work on a sorted unique copy of indices for safety
    let mut idxs: Vec<usize> = outlier_indices.iter().copied().filter(|&i| i < data.len()).collect();
    idxs.sort_unstable();
    idxs.dedup();

    match strat {
        FillStrategy::LocalMedian { half_window } | FillStrategy::LocalMean { half_window } => {
            let use_median = matches!(strat, FillStrategy::LocalMedian { .. });
            for &i in &idxs {
                let start = i.saturating_sub(half_window);
                let end = (i + half_window + 1).min(data.len());
                // Build neighborhood excluding the outlier itself for purity
                let mut local: Vec<f64> = Vec::with_capacity(end - start);
                for (j, &v) in data[start..end].iter().enumerate() {
                    if start + j != i {
                        local.push(v);
                    }
                }
                if local.is_empty() {
                    // fallback: keep original (should be rare)
                    continue;
                }
                let replacement = if use_median {
                    median(&local)
                } else {
                    // mean
                    if local.is_empty() {
                        f64::NAN
                    } else {
                        local.iter().sum::<f64>() / local.len() as f64
                    }
                };
                if replacement.is_finite() {
                    out[i] = replacement;
                }
            }
        }
        FillStrategy::LinearInterp => {
            // Build list of good (x, y) anchors from non-outlier points
            let mut good_x: Vec<f64> = Vec::new();
            let mut good_y: Vec<f64> = Vec::new();
            for (i, &v) in data.iter().enumerate() {
                if !idxs.contains(&i) && v.is_finite() {
                    good_x.push(i as f64);
                    good_y.push(v);
                }
            }
            if good_x.len() < 2 {
                // Not enough anchors — forward/back fill from first/last good
                let first_good = good_y.first().copied().unwrap_or(0.0);
                let last_good = good_y.last().copied().unwrap_or(0.0);
                for &i in &idxs {
                    out[i] = if i < good_x.len() { first_good } else { last_good };
                }
                return out;
            }
            // Interpolate the bad positions
            let bad_x: Vec<f64> = idxs.iter().map(|&i| i as f64).collect();
            let filled = interp_linear(&good_x, &good_y, &bad_x);
            for (k, &i) in idxs.iter().enumerate() {
                if k < filled.len() && filled[k].is_finite() {
                    out[i] = filled[k];
                }
            }
        }
    }

    out
}

/// Convenience: detect + replace in one call (the "dynamics interpolation" entry point).
///
/// Equivalent to `interpolate_outliers(data, &detect_outliers(data, crit), strat)`.
pub fn robust_interpolate(data: &[f64], crit: OutlierCriterion, strat: FillStrategy) -> Vec<f64> {
    let bad = detect_outliers(data, crit);
    interpolate_outliers(data, &bad, strat)
}

/// Time-aware variant of [`robust_interpolate`].
///
/// `times` must be the same length as `data` and strictly increasing.
/// The same criteria and strategies are supported; `Local*` windows are still
/// index-based (sample neighborhood) while `LinearInterp` uses the actual time
/// values for the interpolation anchors (more natural for irregular RR data).
pub fn robust_interpolate_with_times(
    times: &[f64],
    data: &[f64],
    crit: OutlierCriterion,
    strat: FillStrategy,
) -> Vec<f64> {
    if times.len() != data.len() || times.len() < 2 {
        // fall back to pure sample version
        return robust_interpolate(data, crit, strat);
    }
    let bad = detect_outliers(data, crit);
    if bad.is_empty() {
        return data.to_vec();
    }

    match strat {
        FillStrategy::LinearInterp => {
            // Collect good (t, y)
            let mut good_t: Vec<f64> = Vec::new();
            let mut good_y: Vec<f64> = Vec::new();
            for (i, &v) in data.iter().enumerate() {
                if !bad.contains(&i) && v.is_finite() {
                    good_t.push(times[i]);
                    good_y.push(v);
                }
            }
            if good_t.len() < 2 {
                // fallback fill
                let first = good_y.first().copied().unwrap_or(0.0);
                let last = good_y.last().copied().unwrap_or(0.0);
                let mut out = data.to_vec();
                for &i in &bad {
                    out[i] = if i < good_t.len() { first } else { last };
                }
                return out;
            }
            let bad_t: Vec<f64> = bad.iter().map(|&i| times[i]).collect();
            let filled = interp_linear(&good_t, &good_y, &bad_t);
            let mut out = data.to_vec();
            for (k, &i) in bad.iter().enumerate() {
                if k < filled.len() && filled[k].is_finite() {
                    out[i] = filled[k];
                }
            }
            out
        }
        _ => {
            // For local-median/mean we still use index windows (simpler and usually sufficient)
            let mut out = data.to_vec();
            let idxs = bad;
            match strat {
                FillStrategy::LocalMedian { half_window } | FillStrategy::LocalMean { half_window } => {
                    let use_median = matches!(strat, FillStrategy::LocalMedian { .. });
                    for &i in &idxs {
                        let start = i.saturating_sub(half_window);
                        let end = (i + half_window + 1).min(data.len());
                        let mut local: Vec<f64> = Vec::with_capacity(end - start);
                        for (j, &v) in data[start..end].iter().enumerate() {
                            if start + j != i {
                                local.push(v);
                            }
                        }
                        if !local.is_empty() {
                            let replacement = if use_median {
                                median(&local)
                            } else {
                                local.iter().sum::<f64>() / local.len() as f64
                            };
                            if replacement.is_finite() {
                                out[i] = replacement;
                            }
                        }
                    }
                }
                _ => unreachable!(),
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_local_mad() {
        // 10 normal + one big spike
        let mut d: Vec<f64> = (0..20).map(|i| 800.0 + (i as f64) * 0.1).collect();
        d[10] = 2000.0; // obvious outlier
        let bad = detect_outliers(&d, OutlierCriterion::LocalMAD { half_window: 3, k: 4.0 });
        assert!(bad.contains(&10));
    }

    #[test]
    fn test_detect_percent_change() {
        let d = vec![800.0, 810.0, 1000.0, 1010.0]; // big jump at index 2
        let bad = detect_outliers(&d, OutlierCriterion::PercentChange(0.15));
        assert!(bad.contains(&2));
    }

    #[test]
    fn test_robust_local_median_fill() {
        let d = vec![1.0, 1.1, 100.0, 1.2, 1.3];
        let cleaned = robust_interpolate(
            &d,
            OutlierCriterion::Absolute(10.0),
            FillStrategy::LocalMedian { half_window: 1 },
        );
        assert!((cleaned[2] - 1.1).abs() < 0.5 || (cleaned[2] - 1.2).abs() < 0.5);
        assert_eq!(cleaned.len(), d.len());
    }

    #[test]
    fn test_linear_interp_replacement() {
        let d = vec![0.0, 10.0, 999.0, 30.0, 40.0];
        let cleaned = robust_interpolate(&d, OutlierCriterion::Absolute(100.0), FillStrategy::LinearInterp);
        // 999 should be replaced by linear interp between 10 and 30 → ~20
        assert!((cleaned[2] - 20.0).abs() < 1.0);
    }

    #[test]
    fn test_time_aware_linear() {
        let t = vec![0.0, 1.0, 2.0, 10.0, 11.0];
        let d = vec![0.0, 1.0, 999.0, 10.0, 11.0];
        let cleaned =
            robust_interpolate_with_times(&t, &d, OutlierCriterion::Absolute(100.0), FillStrategy::LinearInterp);
        // The outlier at t=2 should be interpolated using real times
        assert!(cleaned[2].is_finite());
        assert!((cleaned[2] - 1.0).abs() < 5.0); // rough
    }
}
