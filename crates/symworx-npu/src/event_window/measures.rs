// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! RMSSD, CV, sample entropy, and the 15 s or 30 s grid for one series.

use alloc::vec::Vec;

use super::{
    SAMPLE_ENTROPY_M,
    SAMPLE_ENTROPY_R_SCALE,
    WINDOW_SEC,
    repair::{
        EventWindow,
        IntervalScore,
        repair_event_window,
        score_event_intervals,
    },
    window_bounds,
};
use crate::{
    NpuError,
    float,
};

/// RMSSD, CV, and sample entropy for one admitted window.
#[derive(Debug, Clone, PartialEq)]
pub struct EventMeasures {
    /// Root mean square of successive differences, in seconds.
    pub rmssd: f64,
    /// Population standard deviation divided by the absolute mean.
    pub cv: f64,
    /// Sample entropy of the repaired intervals, in nats.
    pub sample_entropy: f64,
    /// Mean combined probability of the window, before replacement.
    pub mean_probability: f64,
    /// Number of repaired intervals.
    pub n_intervals: usize,
}

/// Measured intervals and probabilities that close inside one export step.
///
/// These are the detected features, before low-probability gaps are replaced.
/// The mean and standard deviation use the population divisor (`n`). A step
/// with fewer than two intervals has a non-finite standard deviation.
#[derive(Debug, Clone, PartialEq)]
pub struct StepFeatures {
    /// Measured interval for each event that closes in the step, in seconds.
    pub intervals_sec: Vec<f64>,
    /// Combined probability of each interval, from the 180 s score.
    pub probability: Vec<f64>,
    /// Population mean of [`Self::intervals_sec`].
    pub mean_interval: f64,
    /// Population standard deviation of [`Self::intervals_sec`].
    pub std_interval: f64,
    /// Population mean of [`Self::probability`].
    pub mean_probability: f64,
    /// Population standard deviation of [`Self::probability`].
    pub std_probability: f64,
}

/// One instant on the recalculation grid.
#[derive(Debug, Clone, PartialEq)]
pub struct RollingSample {
    /// End of the lookback window, in seconds.
    pub time_sec: f64,
    /// Variability and complexity when that 180 s slice was admitted.
    pub measures: Option<EventMeasures>,
    /// Features and probabilities that close in the step ending at [`Self::time_sec`].
    pub features: Option<StepFeatures>,
}

/// Root mean square of successive differences.
///
/// Matches `symworx-stats`: `len < 3` returns NaN, and the divisor is `len - 1`.
pub fn rmssd(data: &[f64]) -> f64 {
    if data.len() < 3 {
        return f64::NAN;
    }
    let mut sum_sq = 0.0;
    for pair in data.windows(2) {
        let diff = (pair[1] - pair[0]).abs();
        sum_sq += diff * diff;
    }
    float::sqrt(sum_sq / (data.len() - 1) as f64)
}

/// Population coefficient of variation, `std / |mean|`.
///
/// Matches `symworx-stats`. The standard deviation divides by `n`. A zero
/// mean, or fewer than two samples, returns NaN.
pub fn cv(data: &[f64]) -> f64 {
    let sd = population_std(data);
    if data.len() < 2 || !sd.is_finite() {
        return f64::NAN;
    }
    let mean = data.iter().sum::<f64>() / data.len() as f64;
    if mean == 0.0 || !mean.is_finite() {
        return f64::NAN;
    }
    sd / mean.abs()
}

/// RMSSD, CV, and sample entropy of one admitted window.
///
/// Sample entropy uses [`SAMPLE_ENTROPY_M`] and a tolerance of
/// [`SAMPLE_ENTROPY_R_SCALE`] times the population standard deviation.
pub fn event_measures(window: &EventWindow) -> EventMeasures {
    let data = window.intervals_sec.as_slice();
    let sd = population_std(data);
    EventMeasures {
        rmssd: rmssd(data),
        cv: cv(data),
        sample_entropy: crate::sample_entropy(data, SAMPLE_ENTROPY_M, SAMPLE_ENTROPY_R_SCALE * sd),
        mean_probability: window.mean_probability,
        n_intervals: data.len(),
    }
}

/// Repair the prior 180 s at each step and measure the admitted windows.
///
/// `step_sec` is the recalculation interval. [`super::MEASURE_STEP_SEC`] is 30 s and
/// [`super::MEASURE_STEP_FAST_SEC`] is 15 s. The first sample is [`WINDOW_SEC`] after
/// the first event, then every `step_sec` through the last event. Variability
/// and complexity use the prior 180 s. The sample's features are the measured
/// intervals and probabilities that close inside the step, with their mean and
/// standard deviation. A grid time whose 180 s slice is not admitted has
/// `measures: None`. A step with no closing interval has `features: None`.
pub fn rolling_event_measures(
    event_times_sec: &[f64],
    prominences: &[f64],
    step_sec: f64,
) -> Result<Vec<RollingSample>, NpuError> {
    if !step_sec.is_finite() || step_sec <= 0.0 {
        return Err(NpuError::InvalidInput("measure step must be finite and positive"));
    }
    score_event_intervals(event_times_sec, prominences)?;
    let t0 = event_times_sec[0];
    let t_last = event_times_sec[event_times_sec.len() - 1];
    let mut samples = Vec::new();
    let mut t = t0 + WINDOW_SEC;
    while t <= t_last + 1e-9 {
        let (measures, features) = sample_ending_at(event_times_sec, prominences, t, step_sec)?;
        samples.push(RollingSample {
            time_sec: t,
            measures,
            features,
        });
        let next = t + step_sec;
        if next <= t {
            return Err(NpuError::InvalidInput("measure step must advance the clock"));
        }
        t = next;
    }
    Ok(samples)
}

pub fn population_std(data: &[f64]) -> f64 {
    if data.len() < 2 {
        return f64::NAN;
    }
    let n = data.len() as f64;
    let mean = data.iter().sum::<f64>() / n;
    if !mean.is_finite() {
        return f64::NAN;
    }
    let variance = data
        .iter()
        .map(|x| {
            let d = x - mean;
            d * d
        })
        .sum::<f64>()
        / n;
    float::sqrt(variance.max(0.0))
}

fn sample_ending_at(
    event_times_sec: &[f64],
    prominences: &[f64],
    t_end: f64,
    step_sec: f64,
) -> Result<(Option<EventMeasures>, Option<StepFeatures>), NpuError> {
    let Some((first, last)) = window_bounds(event_times_sec, t_end) else {
        return Ok((None, None));
    };
    let times = &event_times_sec[first..=last];
    let prominences = &prominences[first..=last];
    let scores = score_event_intervals(times, prominences)?;
    let features = step_features(&scores, t_end, step_sec);
    let measures = repair_event_window(times, prominences)?.map(|window| event_measures(&window));
    Ok((measures, features))
}

fn step_features(scores: &[IntervalScore], t_end: f64, step_sec: f64) -> Option<StepFeatures> {
    let start = t_end - step_sec;
    let mut intervals_sec = Vec::new();
    let mut probability = Vec::new();
    for score in scores {
        if score.time_sec > start + 1e-9 && score.time_sec <= t_end + 1e-9 {
            intervals_sec.push(score.interval_sec);
            probability.push(score.probability);
        }
    }
    if intervals_sec.is_empty() {
        return None;
    }
    Some(StepFeatures {
        mean_interval: mean(&intervals_sec),
        std_interval: population_std(&intervals_sec),
        mean_probability: mean(&probability),
        std_probability: population_std(&probability),
        intervals_sec,
        probability,
    })
}

fn mean(data: &[f64]) -> f64 {
    if data.is_empty() {
        return f64::NAN;
    }
    data.iter().sum::<f64>() / data.len() as f64
}

#[cfg(test)]
mod tests {
    use super::{
        super::{
            MEASURE_STEP_FAST_SEC,
            MEASURE_STEP_SEC,
            SAMPLE_ENTROPY_M,
            SAMPLE_ENTROPY_R_SCALE,
            repair::repair_event_window,
            support::seconds,
        },
        *,
    };
    use crate::float;

    #[test]
    fn rmssd_matches_the_stats_definition() {
        // diffs 1 and 2, rms = sqrt((1+4)/2)
        let got = rmssd(&[1.0, 2.0, 4.0]);
        assert!((got - float::sqrt(2.5)).abs() < 1e-12);
        assert!(rmssd(&[1.0, 2.0]).is_nan());
        assert_eq!(cv(&[1.0, 1.0, 1.0]), 0.0);
        assert!(cv(&[1.0]).is_nan());
    }

    #[test]
    fn rolling_on_a_regular_series_samples_every_step_and_entropy_is_zero() {
        let (times, prominences) = seconds(401);
        let fast = rolling_event_measures(&times, &prominences, MEASURE_STEP_FAST_SEC).unwrap();
        let slow = rolling_event_measures(&times, &prominences, MEASURE_STEP_SEC).unwrap();
        assert_eq!(fast.first().unwrap().time_sec, 180.0);
        assert_eq!(fast.last().unwrap().time_sec, 390.0);
        assert_eq!(fast.len(), 15);
        assert_eq!(slow.len(), 8);
        assert!(slow.iter().all(|s| s.measures.as_ref().unwrap().sample_entropy == 0.0));
        assert!(slow.iter().all(|s| s.measures.as_ref().unwrap().rmssd == 0.0));
        assert!(slow.iter().all(|s| s.measures.as_ref().unwrap().cv == 0.0));
        let first = slow[0].features.as_ref().unwrap();
        let second = slow[1].features.as_ref().unwrap();
        assert_eq!(first.intervals_sec.len(), 30);
        assert_eq!(second.intervals_sec.len(), 30);
        assert!(first.intervals_sec.iter().all(|v| *v == 1.0));
        assert_eq!(first.std_interval, 0.0);
        assert!((first.mean_interval - 1.0).abs() < 1e-12);
        assert!((first.mean_probability - 1.0).abs() < 1e-12);
        assert_eq!(first.std_probability, 0.0);
        assert_eq!(fast[0].features.as_ref().unwrap().intervals_sec.len(), 15);
    }

    #[test]
    fn rolling_waits_for_180_seconds_and_rejects_a_bad_step() {
        let (times, prominences) = seconds(101);
        assert!(
            rolling_event_measures(&times, &prominences, MEASURE_STEP_SEC)
                .unwrap()
                .is_empty()
        );
        assert!(rolling_event_measures(&times, &prominences, 0.0).is_err());
        assert!(rolling_event_measures(&times, &prominences, f64::NAN).is_err());
    }

    #[test]
    fn rolling_sample_entropy_follows_the_prior_180_seconds() {
        let mut times = vec![0.0];
        for _ in 0..180 {
            times.push(*times.last().unwrap() + 1.0);
        }
        let gaps: Vec<f64> = (0..180).map(|i| 1.0 + 0.02 * float::sin(i as f64 / 8.0)).collect();
        let scale = 180.0 / gaps.iter().sum::<f64>();
        for gap in gaps {
            times.push(*times.last().unwrap() + gap * scale);
        }
        let end = times.len() - 1;
        times[end] = 360.0;
        let prominences = vec![1.0; times.len()];
        let samples = rolling_event_measures(&times, &prominences, 180.0).unwrap();
        assert_eq!(samples.len(), 2);
        assert!((samples[0].time_sec - 180.0).abs() < 1e-9);
        assert_eq!(samples[0].measures.as_ref().unwrap().sample_entropy, 0.0);
        let later = samples[1].measures.as_ref().unwrap();
        assert!(later.sample_entropy > 0.2);
        assert!(later.rmssd > 0.0);
        assert!((samples[1].time_sec - 360.0).abs() < 1e-6);
    }

    #[test]
    fn event_measures_sample_entropy_is_positive_on_a_mild_window() {
        let gaps: Vec<f64> = (0..180).map(|i| 1.0 + 0.05 * float::sin(i as f64 / 10.0)).collect();
        let mut times = vec![0.0];
        for gap in &gaps {
            times.push(*times.last().unwrap() + gap);
        }
        let prominences = vec![1.0; times.len()];
        let window = repair_event_window(&times, &prominences).unwrap().unwrap();
        let measures = event_measures(&window);
        let sd = population_std(&window.intervals_sec);
        let expected = crate::sample_entropy(&window.intervals_sec, SAMPLE_ENTROPY_M, SAMPLE_ENTROPY_R_SCALE * sd);
        assert!((measures.sample_entropy - expected).abs() < 1e-12);
        assert!((measures.sample_entropy - 0.22648990134155392).abs() < 1e-9);
        assert_eq!(measures.n_intervals, 180);
    }
}
