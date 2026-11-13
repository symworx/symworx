// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Per-event probability and in-place repair for one feature window.
//!
//! This is the scalar copy of `symworx-signal`'s event window. The fit, the
//! constants, and the fill match that crate. A malformed series is
//! [`NpuError::InvalidInput`]. A series that can be scored but fails the
//! 180 s span, the mean-probability gate, or the two-sided fill is `Ok(None)`.
//!
//! [`rolling_event_measures`] repeats that repair every 15 s or 30 s on the
//! prior 180 s. An admitted sample has RMSSD, CV, and sample entropy.
//!
//! [`rolling_joint_coupling`] does the same lookback on two series. It trims
//! the longer repaired list from the front, then reports cross-sample entropy
//! and cross-recurrence. `diag_entropy` is in nats.

mod joint;
mod measures;
mod repair;

pub use joint::{
    JointCoupling,
    JointSample,
    joint_coupling,
    rolling_joint_coupling,
    trim_to_same_end,
};
pub use measures::{
    EventMeasures,
    RollingSample,
    StepFeatures,
    cv,
    event_measures,
    population_std,
    rmssd,
    rolling_event_measures,
};
pub use repair::{
    EventWindow,
    IntervalScore,
    interpolate_low_probability,
    peer_fit,
    repair_event_window,
    score_event_intervals,
};

/// Combined probability below this is replaced in place.
pub const POINT_MIN: f64 = 0.6;

/// The mean combined probability must be strictly greater than this.
pub const MEAN_MIN: f64 = 0.8;

/// Minimum time from the first event to the last, in seconds.
pub const WINDOW_SEC: f64 = 180.0;

/// Recalculation cadence, in seconds. Each sample still uses the prior [`WINDOW_SEC`].
pub const MEASURE_STEP_SEC: f64 = 30.0;

/// Shorter recalculation cadence, in seconds. Same 180 s lookback as [`MEASURE_STEP_SEC`].
pub const MEASURE_STEP_FAST_SEC: f64 = 15.0;

/// Embedding dimension for the rolling sample entropy.
pub const SAMPLE_ENTROPY_M: usize = 2;

/// Sample-entropy tolerance as a fraction of the window's population standard deviation.
pub const SAMPLE_ENTROPY_R_SCALE: f64 = 0.2;

/// First and last event indexes in `[t_end - WINDOW_SEC, t_end]`.
fn window_bounds(times: &[f64], t_end: f64) -> Option<(usize, usize)> {
    let start = t_end - WINDOW_SEC;
    let first = times.iter().position(|t| *t >= start - 1e-9)?;
    let last = times.iter().rposition(|t| *t <= t_end + 1e-9)?;
    if last < first { None } else { Some((first, last)) }
}

#[cfg(test)]
mod support {
    use alloc::vec::Vec;

    pub(super) fn uniform_window(n_events: usize, gap: f64, prominence: f64) -> (Vec<f64>, Vec<f64>) {
        let times: Vec<f64> = (0..n_events).map(|i| i as f64 * gap).collect();
        let prominences = vec![prominence; n_events];
        (times, prominences)
    }

    pub(super) fn seconds(n: usize) -> (Vec<f64>, Vec<f64>) {
        uniform_window(n, 1.0, 1.0)
    }
}
