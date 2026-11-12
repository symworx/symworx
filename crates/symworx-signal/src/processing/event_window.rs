// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Per-event probability and in-place repair for one feature window.
//!
//! Each interval is the gap that closes on a detected event (stride peak, R
//! peak, pulse peak). Its probability is the worse of two leave-one-out fits:
//! the interval among the other intervals, and that event's prominence among
//! the other prominences. A fit is `exp(-z² / 8)`, so a point about two
//! sample standard deviations from the others scores near [`POINT_MIN`].
//!
//! Intervals below [`POINT_MIN`] are rewritten from the nearest kept intervals
//! on both sides. The event times stay where they are. The window is returned
//! only when the events span at least [`WINDOW_SEC`] and the mean probability,
//! taken before that rewrite, is greater than [`MEAN_MIN`].
//!
//! RMSSD, CV, and sample entropy are computed by the caller on
//! [`EventWindow::intervals_sec`].

/// Combined probability below this is replaced in place.
pub const POINT_MIN: f64 = 0.6;

/// The mean combined probability must be strictly greater than this.
pub const MEAN_MIN: f64 = 0.8;

/// Minimum time from the first event to the last, in seconds.
pub const WINDOW_SEC: f64 = 180.0;

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
///
/// Returns `None` when fewer than three finite values are supplied.
pub fn peer_fit(values: &[f64]) -> Option<Vec<f64>> {
    let n = values.len();
    if n < 3 || values.iter().any(|v| !v.is_finite()) {
        return None;
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
        let sd = variance.max(0.0).sqrt();
        let score = if sd <= 1e-12 {
            if (x - mean).abs() <= 1e-12 { 1.0 } else { 0.0 }
        } else {
            let z = (x - mean) / sd;
            (-z * z / 8.0).exp()
        };
        scores.push(score);
    }
    Some(scores)
}

/// Score the interval that ends at each event after the first.
///
/// `event_times_sec` and `prominences` are one entry per detected event and
/// must be the same length. Times are strictly increasing. The first event
/// opens the first interval. Its prominence is still part of the detector
/// peer set. Each later event closes one interval.
///
/// Returns `None` when the inputs cannot be scored: fewer than four events,
/// a length mismatch, a non-finite value, or times that are not strictly
/// increasing.
pub fn score_event_intervals(event_times_sec: &[f64], prominences: &[f64]) -> Option<Vec<IntervalScore>> {
    if event_times_sec.len() != prominences.len() || event_times_sec.len() < 4 {
        return None;
    }
    if !event_times_sec.iter().all(|t| t.is_finite()) || !prominences.iter().all(|p| p.is_finite()) {
        return None;
    }
    if event_times_sec.windows(2).any(|w| w[1] <= w[0]) {
        return None;
    }

    let intervals: Vec<f64> = event_times_sec.windows(2).map(|w| w[1] - w[0]).collect();
    let interval_fit = peer_fit(&intervals)?;
    let prominence_fit = peer_fit(prominences)?;

    Some(
        intervals
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
            .collect(),
    )
}

/// Replace values whose probability is below [`POINT_MIN`].
///
/// The new value lies on the line between the nearest kept samples on each
/// side, using `times_sec` as the coordinate. Kept samples are unchanged.
/// Returns `None` when the slices disagree, a sample is non-finite, the times
/// are not strictly increasing, or a low-probability sample has no kept
/// sample on both sides.
pub fn interpolate_low_probability(times_sec: &[f64], values: &[f64], probability: &[f64]) -> Option<Vec<f64>> {
    let n = values.len();
    if n == 0 || times_sec.len() != n || probability.len() != n {
        return None;
    }
    if !times_sec.iter().all(|t| t.is_finite())
        || !values.iter().all(|v| v.is_finite())
        || !probability.iter().all(|p| p.is_finite())
    {
        return None;
    }
    if times_sec.windows(2).any(|w| w[1] <= w[0]) {
        return None;
    }

    let kept: Vec<bool> = probability.iter().map(|p| *p >= POINT_MIN).collect();
    let mut repaired = values.to_vec();
    for i in 0..n {
        if kept[i] {
            continue;
        }
        let left = (0..i).rev().find(|&j| kept[j])?;
        let right = (i + 1..n).find(|&j| kept[j])?;
        let dt = times_sec[right] - times_sec[left];
        if dt <= 0.0 {
            return None;
        }
        let weight = (times_sec[i] - times_sec[left]) / dt;
        repaired[i] = values[left] + weight * (values[right] - values[left]);
    }
    Some(repaired)
}

/// Score a detected-event window and fill intervals below [`POINT_MIN`].
///
/// Returns `None` when the events cannot be scored, the first event and the
/// last are less than [`WINDOW_SEC`] apart, the mean probability is at most
/// [`MEAN_MIN`], or a low-probability interval cannot be filled from both sides.
pub fn repair_event_window(event_times_sec: &[f64], prominences: &[f64]) -> Option<EventWindow> {
    let scores = score_event_intervals(event_times_sec, prominences)?;
    let span_sec = event_times_sec.last()? - event_times_sec.first()?;
    if span_sec < WINDOW_SEC {
        return None;
    }

    let probability: Vec<f64> = scores.iter().map(|s| s.probability).collect();
    let mean_probability = probability.iter().sum::<f64>() / probability.len() as f64;
    if mean_probability <= MEAN_MIN {
        return None;
    }

    let times: Vec<f64> = scores.iter().map(|s| s.time_sec).collect();
    let values: Vec<f64> = scores.iter().map(|s| s.interval_sec).collect();
    let intervals_sec = interpolate_low_probability(&times, &values, &probability)?;
    Some(EventWindow {
        intervals_sec,
        probability,
        mean_probability,
        span_sec,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uniform_window(n_events: usize, gap: f64, prominence: f64) -> (Vec<f64>, Vec<f64>) {
        let times: Vec<f64> = (0..n_events).map(|i| i as f64 * gap).collect();
        let prominences = vec![prominence; n_events];
        (times, prominences)
    }

    #[test]
    fn peer_fit_of_identical_values_is_one() {
        let scores = peer_fit(&[0.9, 0.9, 0.9, 0.9]).unwrap();
        assert!(scores.iter().all(|p| (*p - 1.0).abs() < 1e-12));
    }

    #[test]
    fn peer_fit_marks_a_400ms_interval_among_900ms_peers() {
        // Others are 0.7, 0.9, 1.1 s: mean 0.9, sample sd 0.2. Held-out 0.4 s is z = -2.5.
        let scores = peer_fit(&[0.7, 0.9, 1.1, 0.4]).unwrap();
        let expected = (-2.5_f64 * 2.5 / 8.0).exp();
        assert!((scores[3] - expected).abs() < 1e-12);
        assert!(scores[3] < POINT_MIN);
        assert!(scores[1] > POINT_MIN);
    }

    #[test]
    fn peer_fit_rejects_a_lone_mismatch_when_the_others_have_no_spread() {
        let scores = peer_fit(&[0.9, 0.9, 0.9, 0.4]).unwrap();
        assert_eq!(scores[3], 0.0);
        assert!(scores[..3].iter().all(|p| (*p - 1.0).abs() < 1e-9 || *p > 0.9));
    }

    #[test]
    fn peer_fit_rejects_short_or_non_finite_input() {
        assert!(peer_fit(&[1.0, 2.0]).is_none());
        assert!(peer_fit(&[1.0, 2.0, f64::NAN]).is_none());
    }

    #[test]
    fn score_uses_the_closing_event_and_takes_the_worse_fit() {
        let times = [0.0, 0.7, 1.6, 2.7, 3.1];
        let prominences = [1.0, 1.0, 1.0, 1.0, 1.0];
        let scores = score_event_intervals(&times, &prominences).unwrap();
        assert_eq!(scores.len(), 4);
        assert!((scores[3].interval_sec - 0.4).abs() < 1e-12);
        assert!((scores[3].time_sec - 3.1).abs() < 1e-12);
        assert!((scores[3].detector_fit - 1.0).abs() < 1e-12);
        let expected = (-2.5_f64 * 2.5 / 8.0).exp();
        assert!((scores[3].peer_fit - expected).abs() < 1e-12);
        assert!((scores[3].probability - expected).abs() < 1e-12);
    }

    #[test]
    fn score_rejects_bad_event_lists() {
        assert!(score_event_intervals(&[0.0, 1.0, 2.0], &[1.0, 1.0, 1.0]).is_none());
        assert!(score_event_intervals(&[0.0, 1.0, 2.0, 3.0], &[1.0, 1.0, 1.0]).is_none());
        assert!(score_event_intervals(&[0.0, 1.0, 1.0, 2.0], &[1.0, 1.0, 1.0, 1.0]).is_none());
        assert!(score_event_intervals(&[0.0, 1.0, 2.0, f64::NAN], &[1.0, 1.0, 1.0, 1.0]).is_none());
    }

    #[test]
    fn interpolate_uses_time_and_keeps_every_index() {
        let times = [0.0, 1.0, 3.0];
        let values = [1.0, 9.0, 3.0];
        let probability = [1.0, 0.1, 1.0];
        let repaired = interpolate_low_probability(&times, &values, &probability).unwrap();
        assert_eq!(repaired.len(), 3);
        assert!((repaired[0] - 1.0).abs() < 1e-12);
        assert!((repaired[2] - 3.0).abs() < 1e-12);
        // t = 1 is one third of the way from 0 to 3, between values 1 and 3.
        assert!((repaired[1] - (1.0 + 2.0 / 3.0)).abs() < 1e-12);
    }

    #[test]
    fn interpolate_fills_a_run_from_the_same_anchors() {
        let times = [0.0, 1.0, 2.0, 3.0];
        let values = [1.0, 9.0, 9.0, 5.0];
        let probability = [1.0, 0.0, 0.0, 1.0];
        let repaired = interpolate_low_probability(&times, &values, &probability).unwrap();
        assert!((repaired[1] - (1.0 + 4.0 / 3.0)).abs() < 1e-12);
        assert!((repaired[2] - (1.0 + 8.0 / 3.0)).abs() < 1e-12);
    }

    #[test]
    fn interpolate_keeps_a_point_at_the_cutoff_and_rejects_an_open_end() {
        let kept = interpolate_low_probability(&[0.0, 1.0, 2.0], &[1.0, 2.0, 3.0], &[1.0, POINT_MIN, 1.0]).unwrap();
        assert!((kept[1] - 2.0).abs() < 1e-12);
        assert!(interpolate_low_probability(&[0.0, 1.0, 2.0], &[1.0, 9.0, 3.0], &[0.1, 1.0, 1.0]).is_none());
        assert!(interpolate_low_probability(&[0.0, 1.0, 2.0], &[1.0, 3.0, 9.0], &[1.0, 1.0, 0.1]).is_none());
    }

    #[test]
    fn repair_admits_a_regular_window_at_the_span_boundary() {
        let (times, prominences) = uniform_window(4, 60.0, 1.0);
        let window = repair_event_window(&times, &prominences).unwrap();
        assert!((window.span_sec - 180.0).abs() < 1e-12);
        assert!((window.mean_probability - 1.0).abs() < 1e-12);
        assert_eq!(window.intervals_sec, vec![60.0, 60.0, 60.0]);
    }

    #[test]
    fn repair_rejects_a_short_span() {
        let (times, prominences) = uniform_window(4, 59.9, 1.0);
        assert!(repair_event_window(&times, &prominences).is_none());
    }

    #[test]
    fn repair_fills_one_interior_interval_and_keeps_its_low_probability() {
        let (mut times, prominences) = uniform_window(182, 1.0, 1.0);
        // Pull every later event earlier by 0.6 s, so one gap is 0.4 s and the rest stay 1 s.
        for t in times.iter_mut().skip(40) {
            *t -= 0.6;
        }
        let window = repair_event_window(&times, &prominences).unwrap();
        assert_eq!(window.intervals_sec.len(), 181);
        assert!(window.probability[39] < POINT_MIN);
        assert!((window.intervals_sec[39] - 1.0).abs() < 1e-9);
        assert!(window.mean_probability > MEAN_MIN);
        assert!(window.span_sec >= WINDOW_SEC);
    }

    #[test]
    fn repair_rejects_a_low_probability_interval_on_the_end() {
        let (mut times, prominences) = uniform_window(182, 1.0, 1.0);
        let last = *times.last().unwrap();
        *times.last_mut().unwrap() = last - 0.6;
        assert!(repair_event_window(&times, &prominences).is_none());

        let (mut early, prominences) = uniform_window(182, 1.0, 1.0);
        early[1] -= 0.6;
        for t in early.iter_mut().skip(2) {
            *t -= 0.6;
        }
        // The first gap is now 0.4 s and has no earlier kept interval.
        assert!((early[1] - early[0] - 0.4).abs() < 1e-12);
        assert!(repair_event_window(&early, &prominences).is_none());
    }

    #[test]
    fn repair_rejects_when_the_mean_probability_is_not_above_the_gate() {
        // Four intervals from {1, 10, 100, 1000}. The 1000 s gap is interior.
        // Its leave-one-out fit is far below POINT_MIN, and the mean of the
        // four fits is below MEAN_MIN. Span is well past WINDOW_SEC.
        let intervals = [10.0, 1000.0, 1.0, 100.0];
        let mut times = vec![0.0];
        for gap in intervals {
            times.push(times.last().unwrap() + gap);
        }
        let prominences = vec![1.0; times.len()];
        let scores = score_event_intervals(&times, &prominences).unwrap();
        let mean = scores.iter().map(|s| s.probability).sum::<f64>() / scores.len() as f64;
        assert!(mean <= MEAN_MIN);
        assert!(scores[1].probability < POINT_MIN);
        assert!(repair_event_window(&times, &prominences).is_none());
    }

    #[test]
    fn score_takes_the_worse_of_peer_and_detector() {
        let times = [0.0, 0.7, 1.6, 2.7, 3.1];
        let prominences = [1.0, 1.0, 100.0, 1.0, 1.0];
        let scores = score_event_intervals(&times, &prominences).unwrap();
        assert!(scores[1].peer_fit > POINT_MIN);
        assert_eq!(scores[1].detector_fit, 0.0);
        assert_eq!(scores[1].probability, 0.0);
        let expected = (-2.5_f64 * 2.5 / 8.0).exp();
        assert!((scores[3].peer_fit - expected).abs() < 1e-12);
        assert!(scores[3].detector_fit > scores[3].peer_fit);
        assert!((scores[3].probability - scores[3].peer_fit).abs() < 1e-12);
    }

    /// 181 intervals cycling 0.90, 1.00, 1.10 s. Event 40 has a prominence that
    /// does not fit, so interval 39 is the only low probability.
    fn cycle_window() -> (Vec<f64>, Vec<f64>) {
        let cycle = [0.90_f64, 1.00, 1.10];
        let mut times = vec![0.0];
        for i in 0..181 {
            times.push(times.last().unwrap() + cycle[i % 3]);
        }
        let mut prominences = vec![1.0; times.len()];
        prominences[40] = 1.0e6;
        (times, prominences)
    }

    #[test]
    fn repair_golden_cycle_keeps_the_worse_fit_and_the_time_weighted_gap() {
        let (times, prominences) = cycle_window();
        let scores = score_event_intervals(&times, &prominences).unwrap();
        assert!(scores[39].peer_fit > POINT_MIN);
        assert_eq!(scores[39].detector_fit, 0.0);
        assert_eq!(scores[39].probability, 0.0);
        assert!((scores[39].interval_sec - 0.90).abs() < 1e-12);

        let window = repair_event_window(&times, &prominences).unwrap();
        assert_eq!(window.intervals_sec.len(), 181);
        assert_eq!(window.probability[39], 0.0);
        assert!((window.intervals_sec[38] - 1.10).abs() < 1e-12);
        assert!((window.intervals_sec[40] - 1.00).abs() < 1e-12);
        assert!((window.intervals_sec[39] - 20.0 / 19.0).abs() < 1e-12);
        assert!((window.intervals_sec[39] - scores[39].interval_sec).abs() > 0.1);
        assert!(window.mean_probability > MEAN_MIN);
        assert!((window.span_sec - 180.9).abs() < 1e-9);
    }

    #[test]
    fn repair_fills_an_interval_whose_prominence_does_not_fit() {
        let (times, mut prominences) = uniform_window(182, 1.0, 1.0);
        prominences[40] = 1.0e6;
        let window = repair_event_window(&times, &prominences).unwrap();
        assert!(window.probability[39] < POINT_MIN);
        assert!((window.intervals_sec[39] - 1.0).abs() < 1e-12);
        assert!(window.intervals_sec.iter().all(|v| (*v - 1.0).abs() < 1e-12));
    }
}
