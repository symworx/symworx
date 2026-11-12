// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Scalar recurrence masks, RQA, smoothing, and peak detection.
//!
//! The recurrence and RQA entry points are unchanged. Smoothing and peaks are
//! slice-based `f64` routines with the same decisions as the host filters and
//! `PeakFinder` in `symworx-signal`. Gait and ECG functions are presets on that
//! finder. This crate does not call an NPU.
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
mod float;
mod gait;
mod peaks;
mod recurrence;
mod rqa;
mod smooth;

pub use ecg::{
    EcgDetectParams,
    detect_ecg_r_peaks,
    detect_ecg_r_peaks_with,
};
pub use error::NpuError;
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
pub use smooth::{
    BandpassSpec,
    Biquad,
    Ema,
    moving_average,
};
