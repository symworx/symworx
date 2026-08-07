// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Windowing, segmentation, and RR/tachogram utilities.
//!
//! These sit on top of the lower-level rolling / time segmentation in
//! `symworx-math` and the interpolation/resampling primitives in this crate.
//!
//! Intended for producing aligned 30 s / 60 s feature windows from cleaned
//! (see `outliers`) and resampled event series (e.g. RR intervals) prior to
//! computing RMSSD, sample entropy, etc., and for pairing with external
//! epoch data such as delta power.

use crate::processing::interpolation::interp_linear;

/// Resamples an irregular RR / inter-event series onto a uniform time grid
/// (tachogram) at `target_fs`.
///
/// * `event_times_sec` — strictly increasing event times (seconds), same length as `interval_values_sec`.
/// * `interval_values_sec` — the value associated with each event (typically the RR interval in seconds
///   ending at that event time, or instantaneous rate).
/// * `target_fs` — desired sampling rate of the output regular series (e.g. 4.0 Hz is common for HRV spectral).
///
/// Returns a vector of interpolated values on the new regular grid covering
/// approximately the same total duration. This enables consistent windowed
/// application of feature functions and later spectral work.
///
/// The implementation uses linear interpolation on the provided (time, value) pairs.
pub fn resample_rr_to_tachogram(event_times_sec: &[f64], interval_values_sec: &[f64], target_fs: f64) -> Vec<f64> {
    if event_times_sec.len() < 2 || event_times_sec.len() != interval_values_sec.len() || target_fs <= 0.0 {
        return vec![];
    }

    let t0 = event_times_sec[0];
    let t_last = *event_times_sec.last().unwrap();
    let duration = (t_last - t0).max(0.0);
    if duration <= 0.0 {
        return vec![];
    }

    let n_out = ((duration * target_fs).round() as usize).max(1);
    let x_new: Vec<f64> = (0..n_out)
        .map(|i| t0 + (i as f64) * duration / (n_out.saturating_sub(1).max(1) as f64))
        .collect();

    interp_linear(event_times_sec, interval_values_sec, &x_new)
}

// Future: generic windowed_apply, overlap-aware segmenters for spectral (Welch), etc.
// They can live here and delegate to `symworx_math::series` rolling / time_windows.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_rr_basic() {
        let times = vec![0.0, 1.0, 2.0, 3.0];
        let ivals = vec![1.0, 1.0, 1.0, 1.0];
        let out = resample_rr_to_tachogram(&times, &ivals, 2.0);
        assert!(!out.is_empty());
        // All values should be close to 1.0
        for v in &out {
            assert!((v - 1.0).abs() < 0.1);
        }
    }

    #[test]
    fn test_resample_rr_empty_or_bad() {
        assert!(resample_rr_to_tachogram(&[], &[], 4.0).is_empty());
        let t = vec![0.0, 1.0];
        assert!(resample_rr_to_tachogram(&t, &[1.0], 4.0).is_empty()); // len mismatch
    }
}
