// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Scoring and in-place fill for one event window.

use alloc::vec::Vec;

use super::{
    MEAN_MIN,
    POINT_MIN,
    WINDOW_SEC,
};
use crate::{
    NpuError,
    float,
};

/// Leave-one-out fit of one interval, plus the detector fit of the event that closes it.
#[derive(Debug, Clone, PartialEq)]
pub struct IntervalScore {
    /// Gap from the previous event, in seconds.
    pub interval_sec: f64,
    /// Time of the event that closes this interval, in seconds.
    pub time_sec: f64,
    /// Prominence of the closing event, in the detector's units.
    pub prominence: f64,
    /// Fit of [`Self::interval_sec`] among the other intervals.
    pub peer_fit: f64,
    /// Fit of [`Self::prominence`] among the other event prominences.
    pub detector_fit: f64,
    /// `min(peer_fit, detector_fit)`.
    pub probability: f64,
}

/// Intervals admitted for a window, after low-probability gaps are filled.
#[derive(Debug, Clone, PartialEq)]
pub struct EventWindow {
    /// Interval series, same length and order as [`score_event_intervals`].
    pub intervals_sec: Vec<f64>,
    /// Combined probability of each interval, before replacement.
    pub probability: Vec<f64>,
    /// Mean of [`Self::probability`].
    pub mean_probability: f64,
    /// Time from the first event to the last, in seconds.
    pub span_sec: f64,
}

/// Leave-one-out normal fit of each value against the others.
///
/// `z` uses the mean and the sample standard deviation of every value except
/// the one being scored. The score is `exp(-z² / 8)`. A zero sample deviation
/// scores 1 when the held-out value matches that mean, and 0 when it does not.
pub fn peer_fit(values: &[f64]) -> Result<Vec<f64>, NpuError> {
    let n = values.len();
    if n < 3 || values.iter().any(|v| !v.is_finite()) {
        return Err(NpuError::InvalidInput("peer fit needs at least three finite values"));
    }

    let sum: f64 = values.iter().sum();
    let sum_sq: f64 = values.iter().map(|v| v * v).sum();
    let mut scores = Vec::with_capacity(n);
    for &x in values {
        let others = (n - 1) as f64;
        let sum_others = sum - x;
        let sum_sq_others = sum_sq - x * x;
        let mean = sum_others / others;
        let variance = (sum_sq_others - sum_others * sum_others / others) / (others - 1.0);
        let sd = float::sqrt(variance.max(0.0));
        let score = if sd <= 1e-12 {
            if (x - mean).abs() <= 1e-12 { 1.0 } else { 0.0 }
        } else {
            let z = (x - mean) / sd;
            float::exp(-z * z / 8.0)
        };
        scores.push(score);
    }
    Ok(scores)
}

/// Score the interval that ends at each event after the first.
///
/// `event_times_sec` and `prominences` are one entry per detected event and
/// must be the same length. Times are strictly increasing. The first event
/// opens the first interval. Its prominence is still part of the detector
/// peer set. Each later event closes one interval.
pub fn score_event_intervals(event_times_sec: &[f64], prominences: &[f64]) -> Result<Vec<IntervalScore>, NpuError> {
    if event_times_sec.len() != prominences.len() {
        return Err(NpuError::InvalidInput(
            "event times and prominences must have the same length",
        ));
    }
    if event_times_sec.len() < 4 {
        return Err(NpuError::InvalidInput("event window needs at least four events"));
    }
    if !event_times_sec.iter().all(|t| t.is_finite()) || !prominences.iter().all(|p| p.is_finite()) {
        return Err(NpuError::InvalidInput("event times and prominences must be finite"));
    }
    if event_times_sec.windows(2).any(|w| w[1] <= w[0]) {
        return Err(NpuError::InvalidInput("event times must be strictly increasing"));
    }

    let intervals: Vec<f64> = event_times_sec.windows(2).map(|w| w[1] - w[0]).collect();
    let interval_fit = peer_fit(&intervals)?;
    let prominence_fit = peer_fit(prominences)?;

    Ok(intervals
        .into_iter()
        .enumerate()
        .map(|(i, interval_sec)| {
            let peer = interval_fit[i];
            let detector = prominence_fit[i + 1];
            IntervalScore {
                interval_sec,
                time_sec: event_times_sec[i + 1],
                prominence: prominences[i + 1],
                peer_fit: peer,
                detector_fit: detector,
                probability: peer.min(detector),
            }
        })
        .collect())
}

/// Replace values whose probability is below [`POINT_MIN`].
///
/// The new value lies on the line between the nearest kept samples on each
/// side, using `times_sec` as the coordinate. Kept samples are unchanged.
/// `Ok(None)` means a low-probability sample has no kept sample on both sides.
pub fn interpolate_low_probability(
    times_sec: &[f64],
    values: &[f64],
    probability: &[f64],
) -> Result<Option<Vec<f64>>, NpuError> {
    let n = values.len();
    if n == 0 || times_sec.len() != n || probability.len() != n {
        return Err(NpuError::InvalidInput(
            "probability fill slices must be non-empty and the same length",
        ));
    }
    if !times_sec.iter().all(|t| t.is_finite())
        || !values.iter().all(|v| v.is_finite())
        || !probability.iter().all(|p| p.is_finite())
    {
        return Err(NpuError::InvalidInput("probability fill slices must be finite"));
    }
    if times_sec.windows(2).any(|w| w[1] <= w[0]) {
        return Err(NpuError::InvalidInput(
            "probability fill times must be strictly increasing",
        ));
    }

    let kept: Vec<bool> = probability.iter().map(|p| *p >= POINT_MIN).collect();
    let mut repaired = values.to_vec();
    for i in 0..n {
        if kept[i] {
            continue;
        }
        let Some(left) = (0..i).rev().find(|&j| kept[j]) else {
            return Ok(None);
        };
        let Some(right) = (i + 1..n).find(|&j| kept[j]) else {
            return Ok(None);
        };
        let dt = times_sec[right] - times_sec[left];
        if dt <= 0.0 {
            return Err(NpuError::InvalidInput(
                "probability fill times must be strictly increasing",
            ));
        }
        let weight = (times_sec[i] - times_sec[left]) / dt;
        repaired[i] = values[left] + weight * (values[right] - values[left]);
    }
    Ok(Some(repaired))
}

/// Score a detected-event window and fill intervals below [`POINT_MIN`].
///
/// `Ok(None)` means the events were scored and the window was not admitted:
/// the span is under [`WINDOW_SEC`], the mean probability is at most
/// [`MEAN_MIN`], or a low-probability interval has no kept neighbor on both sides.
pub fn repair_event_window(event_times_sec: &[f64], prominences: &[f64]) -> Result<Option<EventWindow>, NpuError> {
    let scores = score_event_intervals(event_times_sec, prominences)?;
    let span_sec = event_times_sec[event_times_sec.len() - 1] - event_times_sec[0];
    if span_sec < WINDOW_SEC {
        return Ok(None);
    }

    let probability: Vec<f64> = scores.iter().map(|s| s.probability).collect();
    let mean_probability = probability.iter().sum::<f64>() / probability.len() as f64;
    if mean_probability <= MEAN_MIN {
        return Ok(None);
    }

    let times: Vec<f64> = scores.iter().map(|s| s.time_sec).collect();
    let values: Vec<f64> = scores.iter().map(|s| s.interval_sec).collect();
    let Some(intervals_sec) = interpolate_low_probability(&times, &values, &probability)? else {
        return Ok(None);
    };
    Ok(Some(EventWindow {
        intervals_sec,
        probability,
        mean_probability,
        span_sec,
    }))
}

#[cfg(test)]
mod tests {
    use super::{
        super::{
            MEAN_MIN,
            POINT_MIN,
            WINDOW_SEC,
            support::uniform_window,
        },
        *,
    };
    use crate::float;

    /// 181 intervals cycling 0.90, 1.00, 1.10 s. Event 40 has a prominence that
    /// does not fit, so interval 39 is the only low probability.
    fn cycle_window() -> (Vec<f64>, Vec<f64>) {
        let cycle = [0.90_f64, 1.00, 1.10];
        let mut times = vec![0.0];
        for i in 0..181 {
            times.push(times[times.len() - 1] + cycle[i % 3]);
        }
        let mut prominences = vec![1.0; times.len()];
        prominences[40] = 1.0e6;
        (times, prominences)
    }

    #[test]
    fn peer_fit_of_identical_values_is_one() {
        let scores = peer_fit(&[0.9, 0.9, 0.9, 0.9]).unwrap();
        assert!(scores.iter().all(|p| (*p - 1.0).abs() < 1e-12));
    }

    #[test]
    fn peer_fit_marks_a_400ms_interval_among_900ms_peers() {
        let scores = peer_fit(&[0.7, 0.9, 1.1, 0.4]).unwrap();
        let expected = float::exp(-2.5_f64 * 2.5 / 8.0);
        assert!((scores[3] - expected).abs() < 1e-12);
        assert!(scores[3] < POINT_MIN);
        assert!(scores[1] > POINT_MIN);
    }

    #[test]
    fn peer_fit_rejects_a_lone_mismatch_when_the_others_have_no_spread() {
        let scores = peer_fit(&[0.9, 0.9, 0.9, 0.4]).unwrap();
        assert_eq!(scores[3], 0.0);
        assert!(scores[..3].iter().all(|p| *p > 0.9));
    }

    #[test]
    fn peer_fit_rejects_short_or_non_finite_input() {
        assert!(peer_fit(&[1.0, 2.0]).is_err());
        assert!(peer_fit(&[1.0, 2.0, f64::NAN]).is_err());
    }

    #[test]
    fn score_uses_the_closing_event_and_takes_the_worse_fit() {
        let times = [0.0, 0.7, 1.6, 2.7, 3.1];
        let prominences = [1.0, 1.0, 100.0, 1.0, 1.0];
        let scores = score_event_intervals(&times, &prominences).unwrap();
        assert_eq!(scores.len(), 4);
        // Interval 1 is a normal 0.9 s gap closed by the odd prominence.
        assert!(scores[1].peer_fit > POINT_MIN);
        assert_eq!(scores[1].detector_fit, 0.0);
        assert_eq!(scores[1].probability, 0.0);
        // Interval 3 is the 0.4 s gap. Its probability is the peer fit.
        let expected = float::exp(-2.5_f64 * 2.5 / 8.0);
        assert!((scores[3].interval_sec - 0.4).abs() < 1e-12);
        assert!((scores[3].peer_fit - expected).abs() < 1e-12);
        assert!(scores[3].detector_fit > scores[3].peer_fit);
        assert!((scores[3].probability - expected).abs() < 1e-12);
    }

    #[test]
    fn score_rejects_bad_event_lists() {
        assert!(score_event_intervals(&[0.0, 1.0, 2.0], &[1.0, 1.0, 1.0]).is_err());
        assert!(score_event_intervals(&[0.0, 1.0, 2.0, 3.0], &[1.0, 1.0, 1.0]).is_err());
        assert!(score_event_intervals(&[0.0, 1.0, 1.0, 2.0], &[1.0, 1.0, 1.0, 1.0]).is_err());
        assert!(score_event_intervals(&[0.0, 1.0, 2.0, f64::NAN], &[1.0, 1.0, 1.0, 1.0]).is_err());
    }

    #[test]
    fn interpolate_uses_time_and_keeps_every_index() {
        let repaired = interpolate_low_probability(&[0.0, 1.0, 3.0], &[1.0, 9.0, 3.0], &[1.0, 0.1, 1.0])
            .unwrap()
            .unwrap();
        assert_eq!(repaired.len(), 3);
        assert!((repaired[0] - 1.0).abs() < 1e-12);
        assert!((repaired[2] - 3.0).abs() < 1e-12);
        assert!((repaired[1] - (1.0 + 2.0 / 3.0)).abs() < 1e-12);
    }

    #[test]
    fn interpolate_fills_a_run_from_the_same_anchors() {
        let repaired = interpolate_low_probability(&[0.0, 1.0, 2.0, 3.0], &[1.0, 9.0, 9.0, 5.0], &[1.0, 0.0, 0.0, 1.0])
            .unwrap()
            .unwrap();
        assert!((repaired[1] - (1.0 + 4.0 / 3.0)).abs() < 1e-12);
        assert!((repaired[2] - (1.0 + 8.0 / 3.0)).abs() < 1e-12);
    }

    #[test]
    fn interpolate_keeps_a_point_at_the_cutoff_and_rejects_an_open_end() {
        let kept = interpolate_low_probability(&[0.0, 1.0, 2.0], &[1.0, 2.0, 3.0], &[1.0, POINT_MIN, 1.0])
            .unwrap()
            .unwrap();
        assert!((kept[1] - 2.0).abs() < 1e-12);
        assert!(
            interpolate_low_probability(&[0.0, 1.0, 2.0], &[1.0, 9.0, 3.0], &[0.1, 1.0, 1.0])
                .unwrap()
                .is_none()
        );
        assert!(
            interpolate_low_probability(&[0.0, 1.0, 2.0], &[1.0, 3.0, 9.0], &[1.0, 1.0, 0.1])
                .unwrap()
                .is_none()
        );
        assert!(interpolate_low_probability(&[0.0, 1.0], &[1.0], &[1.0]).is_err());
    }

    #[test]
    fn repair_admits_a_regular_window_at_the_span_boundary() {
        let (times, prominences) = uniform_window(4, 60.0, 1.0);
        let window = repair_event_window(&times, &prominences).unwrap().unwrap();
        assert!((window.span_sec - 180.0).abs() < 1e-12);
        assert!((window.mean_probability - 1.0).abs() < 1e-12);
        assert_eq!(window.intervals_sec, vec![60.0, 60.0, 60.0]);
    }

    #[test]
    fn repair_rejects_a_short_span() {
        let (times, prominences) = uniform_window(4, 59.9, 1.0);
        assert!(repair_event_window(&times, &prominences).unwrap().is_none());
    }

    #[test]
    fn repair_fills_one_interior_interval_and_keeps_its_low_probability() {
        let (mut times, prominences) = uniform_window(182, 1.0, 1.0);
        for t in times.iter_mut().skip(40) {
            *t -= 0.6;
        }
        let window = repair_event_window(&times, &prominences).unwrap().unwrap();
        assert_eq!(window.intervals_sec.len(), 181);
        assert!(window.probability[39] < POINT_MIN);
        assert!((window.intervals_sec[39] - 1.0).abs() < 1e-9);
        assert!(window.mean_probability > MEAN_MIN);
        assert!(window.span_sec >= WINDOW_SEC);
    }

    #[test]
    fn repair_rejects_a_low_probability_interval_on_the_end() {
        let (mut times, prominences) = uniform_window(182, 1.0, 1.0);
        let last_i = times.len() - 1;
        times[last_i] -= 0.6;
        assert!(repair_event_window(&times, &prominences).unwrap().is_none());

        let (mut early, prominences) = uniform_window(182, 1.0, 1.0);
        early[1] -= 0.6;
        for t in early.iter_mut().skip(2) {
            *t -= 0.6;
        }
        assert!((early[1] - early[0] - 0.4).abs() < 1e-12);
        assert!(repair_event_window(&early, &prominences).unwrap().is_none());
    }

    #[test]
    fn repair_rejects_when_the_mean_probability_is_not_above_the_gate() {
        let intervals = [10.0, 1000.0, 1.0, 100.0];
        let mut times = vec![0.0];
        for gap in intervals {
            times.push(times[times.len() - 1] + gap);
        }
        let prominences = vec![1.0; times.len()];
        let scores = score_event_intervals(&times, &prominences).unwrap();
        let mean = scores.iter().map(|s| s.probability).sum::<f64>() / scores.len() as f64;
        assert!(mean <= MEAN_MIN);
        assert!(scores[1].probability < POINT_MIN);
        assert!(repair_event_window(&times, &prominences).unwrap().is_none());
    }

    #[test]
    fn repair_fills_an_interval_whose_prominence_does_not_fit() {
        let (times, mut prominences) = uniform_window(182, 1.0, 1.0);
        prominences[40] = 1.0e6;
        let window = repair_event_window(&times, &prominences).unwrap().unwrap();
        assert!(window.probability[39] < POINT_MIN);
        assert!((window.intervals_sec[39] - 1.0).abs() < 1e-12);
        assert!(window.intervals_sec.iter().all(|v| (*v - 1.0).abs() < 1e-12));
    }

    #[test]
    fn repair_golden_cycle_keeps_the_worse_fit_and_the_time_weighted_gap() {
        let (times, prominences) = cycle_window();
        let scores = score_event_intervals(&times, &prominences).unwrap();
        assert!(scores[39].peer_fit > POINT_MIN);
        assert_eq!(scores[39].detector_fit, 0.0);
        assert_eq!(scores[39].probability, 0.0);
        assert!((scores[39].interval_sec - 0.90).abs() < 1e-12);

        let window = repair_event_window(&times, &prominences).unwrap().unwrap();
        assert_eq!(window.intervals_sec.len(), 181);
        assert_eq!(window.probability[39], 0.0);
        // Anchors are the neighboring gaps 1.10 s and 1.00 s. The filled value
        // is 20/19 s, which is not the 0.90 s time gap.
        assert!((window.intervals_sec[38] - 1.10).abs() < 1e-12);
        assert!((window.intervals_sec[40] - 1.00).abs() < 1e-12);
        assert!((window.intervals_sec[39] - 20.0 / 19.0).abs() < 1e-12);
        assert!((window.intervals_sec[39] - scores[39].interval_sec).abs() > 0.1);
        assert!(window.mean_probability > MEAN_MIN);
        assert!((window.span_sec - 180.9).abs() < 1e-9);
    }
}
