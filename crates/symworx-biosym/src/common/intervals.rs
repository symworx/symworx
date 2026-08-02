// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

/// Peak timing and derived interval/rate series.
///
/// This type is intentionally general (used for RR intervals in PPG, breath
/// intervals in respiration, stride intervals in gait, etc.) and lives in the
/// crate root `common` layer so that domains (and future
/// domains) can share it without creating cross-dependencies.
///
/// Interval derivation now delegates to `symworx_core::math::series` per
/// project guidelines (single source of truth for successive differences).
use symworx_core::math::series::successive_differences;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct IntervalSeries {
    pub peak_indices: Vec<usize>,
    pub peak_times: Vec<f64>,
    pub intervals_sec: Vec<f64>,
    pub instantaneous_rates: Vec<f64>,
}

impl IntervalSeries {
    /// Build interval and rate series from peak indices and sampling rate.
    pub fn from_peak_indices(indices: &[usize], fs: f64) -> Self {
        if indices.is_empty() || fs <= 0.0 {
            return Self::default();
        }

        let peak_times: Vec<f64> = indices.iter().map(|&i| i as f64 / fs).collect();
        let intervals_sec = successive_differences(&peak_times);
        let instantaneous_rates: Vec<f64> = intervals_sec.iter().map(|&dt| 60.0 / dt).collect();

        Self {
            peak_indices: indices.to_vec(),
            peak_times,
            intervals_sec,
            instantaneous_rates,
        }
    }

    /// Build from absolute event times in seconds (e.g. stride times from gait generation
    /// or CPG, or any time-based events where we don't have underlying sample indices).
    ///
    /// peak_indices will be empty; peak_times, intervals, and rates are populated.
    pub fn from_times(times: &[f64]) -> Self {
        if times.len() < 2 {
            return Self::default();
        }
        let peak_times = times.to_vec();
        let intervals_sec = successive_differences(&peak_times);
        let instantaneous_rates: Vec<f64> = intervals_sec
            .iter()
            .map(|&dt| if dt > 0.0 { 60.0 / dt } else { f64::NAN })
            .collect();

        Self {
            peak_indices: vec![],
            peak_times,
            intervals_sec,
            instantaneous_rates,
        }
    }

    /// Mean event rate over the full recording window (events per minute).
    /// Supports both index-based (from sampled signals) and pure time-based events.
    ///
    /// Uses the number of inter-event *intervals* (i.e. n_events - 1) so that
    /// a recording with events spanning D seconds and I intervals reports
    /// rate = (I / D) * 60. This matches the convention in mean_instantaneous_rate
    /// and the "n_strides = intervals.len()" used elsewhere in gait stats.
    pub fn mean_rate_over_window(&self, duration_sec: f64) -> f64 {
        if duration_sec <= 0.0 {
            return f64::NAN;
        }
        // Prefer the derived intervals count when present (normal case after
        // from_times or from_peak_indices with >=2 events).
        if !self.intervals_sec.is_empty() {
            return 60.0 * self.intervals_sec.len() as f64 / duration_sec;
        }
        // Fallback for manually constructed or edge-case series that have
        // event times but no intervals populated yet.
        let n = if !self.peak_indices.is_empty() {
            self.peak_indices.len()
        } else if !self.peak_times.is_empty() {
            self.peak_times.len()
        } else {
            return f64::NAN;
        };
        if n < 2 {
            return f64::NAN;
        }
        60.0 * (n - 1) as f64 / duration_sec
    }

    /// Mean rate from consecutive intervals (events per minute).
    pub fn mean_instantaneous_rate(&self) -> f64 {
        if self.instantaneous_rates.is_empty() {
            return f64::NAN;
        }
        self.instantaneous_rates.iter().sum::<f64>() / self.instantaneous_rates.len() as f64
    }

    /// Inter-event intervals in seconds (e.g. RR intervals for PPG).
    pub fn as_intervals_sec(&self) -> &[f64] {
        &self.intervals_sec
    }

    /// Split consecutive intervals into even- and odd-indexed phases.
    ///
    /// Useful when peaks alternate inspiration/expiration.
    pub fn alternating_phase_intervals(&self) -> (Vec<f64>, Vec<f64>) {
        let mut even = Vec::new();
        let mut odd = Vec::new();
        for (i, &interval) in self.intervals_sec.iter().enumerate() {
            if i.is_multiple_of(2) {
                even.push(interval);
            } else {
                odd.push(interval);
            }
        }
        (even, odd)
    }
}

/// Convenience wrapper around the core robust ("dynamics") interpolation for
/// cleaning RR / inter-event interval series prior to windowed HRV or entropy
/// analysis.
///
/// See `symworx_core::signal::processing::{OutlierCriterion, FillStrategy, robust_interpolate}`
/// (also re-exported via `symworx_core`) for the full set of options (LocalMAD,
/// PercentChange, LocalMedian / LinearInterp, etc.).
///
/// Example for 4 h sleep-restricted bout preprocessing:
/// ```ignore
/// let cleaned = clean_intervals(&raw_rr_sec, OutlierCriterion::PercentChange(0.20), FillStrategy::LinearInterp);
/// ```
pub fn clean_intervals(
    intervals_sec: &[f64],
    crit: symworx_core::signal::processing::OutlierCriterion,
    strat: symworx_core::signal::processing::FillStrategy,
) -> Vec<f64> {
    symworx_core::signal::processing::robust_interpolate(intervals_sec, crit, strat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_peak_indices_computes_intervals_and_rates() {
        let series = IntervalSeries::from_peak_indices(&[10, 20, 30], 10.0);
        assert_eq!(series.peak_times, vec![1.0, 2.0, 3.0]);
        assert_eq!(series.intervals_sec, vec![1.0, 1.0]);
        assert_eq!(series.instantaneous_rates, vec![60.0, 60.0]);
    }

    #[test]
    fn empty_peaks_returns_default() {
        let series = IntervalSeries::from_peak_indices(&[], 100.0);
        assert!(series.peak_indices.is_empty());
    }

    #[test]
    fn alternating_phase_intervals_splits_even_odd() {
        let series = IntervalSeries::from_peak_indices(&[0, 10, 20, 30], 10.0);
        let (even, odd) = series.alternating_phase_intervals();
        assert_eq!(even, vec![1.0, 1.0]);
        assert_eq!(odd, vec![1.0]);
    }

    #[test]
    fn from_times_works_for_gait_like_events() {
        let times = vec![0.0, 1.1, 2.2, 3.4];
        let series = IntervalSeries::from_times(&times);
        assert!(series.peak_indices.is_empty());
        assert_eq!(series.peak_times, times);
        // Use tolerant compare because 1.1 and 1.2 are not exactly representable
        let expected = vec![1.1, 1.1, 1.2];
        assert_eq!(series.intervals_sec.len(), expected.len());
        for (a, b) in series.intervals_sec.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 1e-9, "interval diff too large: {a} vs {b}");
        }
        assert!((series.mean_instantaneous_rate() - (60.0 / 1.1 + 60.0 / 1.1 + 60.0 / 1.2) / 3.0).abs() < 1e-9);
    }

    #[test]
    fn mean_rate_over_window_supports_time_based() {
        let series = IntervalSeries::from_times(&[0.0, 1.0, 2.0, 3.0]);
        // 3 intervals over 3s window -> 60 events/min
        assert!((series.mean_rate_over_window(3.0) - 60.0).abs() < 1e-9);
        let idx_series = IntervalSeries::from_peak_indices(&[0, 100, 200], 100.0);
        assert!((idx_series.mean_rate_over_window(2.0) - 60.0).abs() < 1e-9);
    }
}
