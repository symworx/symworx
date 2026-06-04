// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

/// Peak timing and derived interval/rate series.
///
/// This type is intentionally general (used for RR intervals in PPG, breath
/// intervals in respiration, stride intervals in gait, etc.) and lives in the
/// crate root `common` layer so that `physiology` and `biomechanics` (and future
/// domains) can share it without creating cross-dependencies.
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
        let intervals_sec: Vec<f64> = peak_times.windows(2).map(|w| w[1] - w[0]).collect();
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
        let intervals_sec: Vec<f64> = peak_times.windows(2).map(|w| w[1] - w[0]).collect();
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
    pub fn mean_rate_over_window(&self, duration_sec: f64) -> f64 {
        if duration_sec <= 0.0 {
            return f64::NAN;
        }
        let n = if !self.peak_indices.is_empty() {
            self.peak_indices.len()
        } else if !self.peak_times.is_empty() {
            self.peak_times.len()
        } else {
            return f64::NAN;
        };
        60.0 * n as f64 / duration_sec
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
    /// Useful when peaks alternate inspiration/expiration (legacy biosym convention).
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
        assert_eq!(series.intervals_sec, vec![1.1, 1.1, 1.2]);
        assert!(
            (series.mean_instantaneous_rate() - (60.0 / 1.1 + 60.0 / 1.1 + 60.0 / 1.2) / 3.0).abs()
                < 1e-9
        );
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
