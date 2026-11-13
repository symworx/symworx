// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Scalar recurrence masks, RQA, smoothing, and peak detection.
//!
//! The recurrence and RQA entry points are unchanged. Smoothing and peaks are
//! slice-based `f64` routines with the same decisions as the host filters and
//! `PeakFinder` in `symworx-signal`. Gait and ECG functions are presets on that
//! finder. The event window matches `symworx-signal`'s repair: leave-one-out
//! probability, a 180 s span, and in-place fill below 0.6. Sample entropy is
//! the scalar copy of `symworx-dynamics`. This crate does not call an NPU.
//!
//! The default `std` feature is the host build, including a Raspberry Pi Zero
//! (`arm-unknown-linux-gnueabihf`).
//! `--no-default-features` is `no_std` plus `alloc` for the Arduino Nano 33 BLE
//! (`thumbv7em-none-eabihf`). The firmware that provides the allocator stays
//! outside this crate.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

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
    peer_fit,
    repair_event_window,
    rmssd,
    rolling_event_measures,
    score_event_intervals,
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
    recurrence_mask,
};
pub use rqa::{
    RqaParams,
    RqaStats,
    rqa,
};
pub use sample_entropy::sample_entropy;
pub use smooth::{
    BandpassSpec,
    Biquad,
    Ema,
    moving_average,
};
