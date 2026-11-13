// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Cross-sample entropy and cross-recurrence on two repaired windows.

use alloc::vec::Vec;

use super::{
    SAMPLE_ENTROPY_M,
    SAMPLE_ENTROPY_R_SCALE,
    WINDOW_SEC,
    measures::population_std,
    repair::{
        repair_event_window,
        score_event_intervals,
    },
    window_bounds,
};
use crate::NpuError;

/// Cross-sample entropy and cross-recurrence for one admitted pair.
///
/// Line metrics come from [`crate::RqaStats`], so `diag_entropy` is in nats.
#[derive(Debug, Clone, PartialEq)]
pub struct JointCoupling {
    /// Length of each trimmed interval series.
    pub n_intervals: usize,
    /// [`crate::cross_sample_entropy`] of the trimmed intervals, in nats.
    pub cross_sample_entropy: f64,
    /// Fraction of recurrent pairs.
    pub rr: f64,
    /// Fraction of recurrent points in diagonal lines.
    pub det: f64,
    /// Fraction of recurrent points in vertical lines.
    pub lam: f64,
    /// Mean diagonal line length.
    pub mean_diag: f64,
    /// Entropy of the diagonal line length histogram, in nats.
    pub diag_entropy: f64,
}

/// One instant on the joint recalculation grid.
#[derive(Debug, Clone, PartialEq)]
pub struct JointSample {
    /// End of the lookback window, in seconds.
    pub time_sec: f64,
    /// Coupling when both 180 s slices were admitted and long enough to embed.
    pub coupling: Option<JointCoupling>,
}

/// Drop oldest intervals from the longer slice until the lengths match.
pub fn trim_to_same_end(left: &[f64], right: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let n = left.len().min(right.len());
    (left[left.len() - n..].to_vec(), right[right.len() - n..].to_vec())
}

/// Repair both sides ending at `t_end` and measure the trimmed intervals.
///
/// Each side is the prior [`WINDOW_SEC`]. `Ok(None)` means a side was not
/// admitted, or fewer than two intervals remain after the trim. A malformed
/// series is [`NpuError::InvalidInput`].
///
/// Tolerance is [`SAMPLE_ENTROPY_R_SCALE`] times the population SD of the two
/// trimmed series concatenated. Cross-recurrence uses that value as the
/// Euclidean radius, with `tau = 1` and Theiler 0.
pub fn joint_coupling(
    left_times_sec: &[f64],
    left_prominences: &[f64],
    right_times_sec: &[f64],
    right_prominences: &[f64],
    t_end: f64,
) -> Result<Option<JointCoupling>, NpuError> {
    coupling_ending_at(
        left_times_sec,
        left_prominences,
        right_times_sec,
        right_prominences,
        t_end,
    )
}

/// Repair both sides on the shared grid.
///
/// The first sample is [`WINDOW_SEC`] after the later start. The last is the
/// earlier end. [`super::MEASURE_STEP_SEC`] is 30 s and [`super::MEASURE_STEP_FAST_SEC`] is
/// 15 s. A step either side does not admit stays on the grid with
/// `coupling: None`. An overlap shorter than 180 s is an empty list.
pub fn rolling_joint_coupling(
    left_times_sec: &[f64],
    left_prominences: &[f64],
    right_times_sec: &[f64],
    right_prominences: &[f64],
    step_sec: f64,
) -> Result<Vec<JointSample>, NpuError> {
    if !step_sec.is_finite() || step_sec <= 0.0 {
        return Err(NpuError::InvalidInput("measure step must be finite and positive"));
    }
    score_event_intervals(left_times_sec, left_prominences)?;
    score_event_intervals(right_times_sec, right_prominences)?;
    let start = left_times_sec[0].max(right_times_sec[0]) + WINDOW_SEC;
    let end = left_times_sec[left_times_sec.len() - 1].min(right_times_sec[right_times_sec.len() - 1]);
    let mut samples = Vec::new();
    let mut t = start;
    while t <= end + 1e-9 {
        let coupling = coupling_ending_at(left_times_sec, left_prominences, right_times_sec, right_prominences, t)?;
        samples.push(JointSample { time_sec: t, coupling });
        let next = t + step_sec;
        if next <= t {
            return Err(NpuError::InvalidInput("measure step must advance the clock"));
        }
        t = next;
    }
    Ok(samples)
}

fn coupling_ending_at(
    left_times_sec: &[f64],
    left_prominences: &[f64],
    right_times_sec: &[f64],
    right_prominences: &[f64],
    t_end: f64,
) -> Result<Option<JointCoupling>, NpuError> {
    let Some((left_first, left_last)) = window_bounds(left_times_sec, t_end) else {
        return Ok(None);
    };
    let Some((right_first, right_last)) = window_bounds(right_times_sec, t_end) else {
        return Ok(None);
    };
    let Some(left) = repair_event_window(
        &left_times_sec[left_first..=left_last],
        &left_prominences[left_first..=left_last],
    )?
    else {
        return Ok(None);
    };
    let Some(right) = repair_event_window(
        &right_times_sec[right_first..=right_last],
        &right_prominences[right_first..=right_last],
    )?
    else {
        return Ok(None);
    };
    let (left_iv, right_iv) = trim_to_same_end(&left.intervals_sec, &right.intervals_sec);
    if left_iv.len() < 2 {
        return Ok(None);
    }
    let mut pooled = Vec::with_capacity(left_iv.len() + right_iv.len());
    pooled.extend_from_slice(&left_iv);
    pooled.extend_from_slice(&right_iv);
    let r = SAMPLE_ENTROPY_R_SCALE * population_std(&pooled);
    let cross_sample_entropy = crate::cross_sample_entropy(&left_iv, &right_iv, SAMPLE_ENTROPY_M, r);
    let stats = match crate::crqa(&left_iv, &right_iv, SAMPLE_ENTROPY_M, 1, r, 0) {
        Ok(stats) => stats,
        Err(NpuError::InvalidInput(_)) => return Ok(None),
        Err(err) => return Err(err),
    };
    Ok(Some(JointCoupling {
        n_intervals: left_iv.len(),
        cross_sample_entropy,
        rr: stats.rr,
        det: stats.det,
        lam: stats.lam,
        mean_diag: stats.mean_diag,
        diag_entropy: stats.diag_entropy,
    }))
}

#[cfg(test)]
mod tests {
    use super::{
        super::{
            MEASURE_STEP_SEC,
            SAMPLE_ENTROPY_M,
            SAMPLE_ENTROPY_R_SCALE,
            WINDOW_SEC,
            measures::population_std,
            repair::repair_event_window,
            support::seconds,
        },
        *,
    };
    use crate::float;

    #[test]
    fn trim_keeps_the_newest_shared_count() {
        let (left, right) = trim_to_same_end(&[1.0, 2.0, 3.0, 4.0], &[8.0, 9.0]);
        assert_eq!(left, vec![3.0, 4.0]);
        assert_eq!(right, vec![8.0, 9.0]);
    }

    #[test]
    fn joint_coupling_trims_the_longer_admitted_window() {
        let (left_times, left_prom) = mild_times(180, 1.0);
        let (right_times, right_prom) = mild_times(120, 1.5);
        let coupling = joint_coupling(&left_times, &left_prom, &right_times, &right_prom, 180.0)
            .unwrap()
            .unwrap();
        let left = repair_event_window(&left_times, &left_prom).unwrap().unwrap();
        let right = repair_event_window(&right_times, &right_prom).unwrap().unwrap();
        let (left_iv, right_iv) = trim_to_same_end(&left.intervals_sec, &right.intervals_sec);
        assert!(left.intervals_sec.len() != right.intervals_sec.len());
        assert_eq!(coupling.n_intervals, left_iv.len());
        assert_eq!(left_iv[0], left.intervals_sec[left.intervals_sec.len() - left_iv.len()]);
        let mut pooled = left_iv.clone();
        pooled.extend_from_slice(&right_iv);
        let r = SAMPLE_ENTROPY_R_SCALE * population_std(&pooled);
        let expected = crate::cross_sample_entropy(&left_iv, &right_iv, SAMPLE_ENTROPY_M, r);
        assert!((coupling.cross_sample_entropy - expected).abs() < 1e-12);
        let stats = crate::crqa(&left_iv, &right_iv, SAMPLE_ENTROPY_M, 1, r, 0).unwrap();
        assert_eq!(coupling.rr, stats.rr);
        assert_eq!(coupling.det, stats.det);
        assert_eq!(coupling.lam, stats.lam);
        assert_eq!(coupling.mean_diag, stats.mean_diag);
        assert_eq!(coupling.diag_entropy, stats.diag_entropy);
    }

    #[test]
    fn joint_coupling_is_absent_when_one_side_is_shorter_than_180_seconds() {
        let (left_times, left_prom) = seconds(181);
        let (right_times, right_prom) = seconds(50);
        assert!(
            joint_coupling(&left_times, &left_prom, &right_times, &right_prom, 180.0)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn rolling_joint_coupling_waits_until_both_sides_have_180_seconds() {
        let (left_times, left_prom) = seconds(401);
        let (mut right_times, right_prom) = seconds(301);
        for t in &mut right_times {
            *t += 30.0;
        }
        let samples =
            rolling_joint_coupling(&left_times, &left_prom, &right_times, &right_prom, MEASURE_STEP_SEC).unwrap();
        assert_eq!(samples.first().unwrap().time_sec, 210.0);
        assert_eq!(samples.last().unwrap().time_sec, 330.0);
        assert!(samples.iter().all(|sample| sample.coupling.is_some()));

        let (short_times, short_prom) = seconds(101);
        let empty =
            rolling_joint_coupling(&left_times, &left_prom, &short_times, &short_prom, MEASURE_STEP_SEC).unwrap();
        assert!(empty.is_empty());
        assert!(rolling_joint_coupling(&left_times, &left_prom, &right_times, &right_prom, 0.0).is_err());
    }

    fn mild_times(n_gaps: usize, base: f64) -> (Vec<f64>, Vec<f64>) {
        let gaps: Vec<f64> = (0..n_gaps).map(|i| base + 0.05 * float::sin(i as f64 / 10.0)).collect();
        let scale = WINDOW_SEC / gaps.iter().sum::<f64>();
        let mut times = vec![0.0];
        for gap in gaps {
            times.push(*times.last().unwrap() + gap * scale);
        }
        let end = times.len() - 1;
        times[end] = WINDOW_SEC;
        let prominences = vec![1.0; times.len()];
        (times, prominences)
    }
}
