// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Scalar recurrence, RQA, smoothing, and peak detection.
//!
//! Smoothing and peaks follow the host filters and `PeakFinder` in
//! `symworx-signal`. Gait and ECG are presets on that finder. The event
//! window matches that crate's repair: leave-one-out probability, a 180 s
//! span, and in-place fill below 0.6. Sample entropy, cross-sample entropy, and
//! detrended fluctuation analysis match `symworx-dynamics`. DFA uses the same
//! stacked boxes, default windows `4..=16` and `16..=64`, and `min_repeats`
//! of 4. Cross recurrence keeps the pair at a shared index
//! when the Theiler window is 0. `joint_coupling` runs both measures after the
//! longer repaired window is trimmed from the front. This crate does not call
//! an NPU.
//!
//! The default `std` feature is the host build, including a Raspberry Pi Zero
//! (`arm-unknown-linux-gnueabihf`).
//! `--no-default-features` is `no_std` plus `alloc` for the Arduino Nano 33 BLE
//! (`thumbv7em-none-eabihf`). The firmware that provides the allocator stays
//! outside this crate.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod dfa;
mod ecg;
mod error;
mod event_window;
mod float;
mod gait;
mod peaks;
mod recurrence;
mod rqa;
mod sample_entropy;
mod smooth;

pub use dfa::{
    DfaConfig,
    DfaFit,
    DfaResult,
    dfa,
};
pub use ecg::{
    EcgDetectParams,
    detect_ecg_r_peaks,
    detect_ecg_r_peaks_with,
};
pub use error::NpuError;
pub use event_window::{
    EventMeasures,
    EventWindow,
    IntervalScore,
    JointCoupling,
    JointSample,
    MEAN_MIN,
    MEASURE_STEP_FAST_SEC,
    MEASURE_STEP_SEC,
    POINT_MIN,
    RollingSample,
    SAMPLE_ENTROPY_M,
    SAMPLE_ENTROPY_R_SCALE,
    StepFeatures,
    WINDOW_SEC,
    cv,
    event_measures,
    interpolate_low_probability,
    joint_coupling,
    peer_fit,
    repair_event_window,
    rmssd,
    rolling_event_measures,
    rolling_joint_coupling,
    score_event_intervals,
    trim_to_same_end,
};
pub use gait::{
    GaitQuality,
    detect_gait_peaks,
    detect_gait_peaks_with_quality,
};
pub use peaks::{
    Peak,
    PeakFinder,
    sample_distance,
};
pub use recurrence::{
    RecurrenceMask,
    Trajectory,
    cross_recurrence_mask,
    embed,
    recurrence_mask,
};
pub use rqa::{
    RqaParams,
    RqaStats,
    crqa,
    rqa,
};
pub use sample_entropy::{
    cross_sample_entropy,
    sample_entropy,
};
pub use smooth::{
    BandpassSpec,
    Biquad,
    Ema,
    moving_average,
};
