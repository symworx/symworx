// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Biomechanics is organized as peer domain modules (currently only `gait`).
//! Each domain (gait, future counter_movement_jump, pedaling, etc.) owns its
//! analysis, processing, params, data, and generation. See `gait/` for the
//! pattern. Shared processing lives at the crate root under `common::processing`.

pub mod gait;

pub use gait::{
    GaitAnalysis,
    GaitData,
    GaitParams,
    GaitSignalQuality,
    GaitStats,
    IntervalSeries,
    analyze_gait,
    analyze_gait_from_times,
    analyze_gait_signal,
    analyze_gait_signal_with_quality,
    compute_gait_stats,
    detect_gait_strides,
    detect_gait_strides_with,
    detect_gait_strides_with_quality,
    gait_default_bandpass,
    gait_peak_finder,
    gait_processing_for_quality,
    gait_processing_high,
    gait_processing_moderate,
    gait_processing_poor,
    gait_processing_reference,
};
