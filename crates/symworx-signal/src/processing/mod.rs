// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Signal processing utilities.
//!
//! Core tools for interpolation, normalization, resampling, decimation
//! (for visualization), peak detection, feature extraction, robust outlier
//! interpolation ("dynamics interpolation"), and windowing / RR-tachogram
//! helpers (supporting 30 s / 60 s feature windows for HRV complexity + delta
//! alignment).

/// Decimation algorithms.
pub mod decimate;

/// Interpolation algorithms.
pub mod interpolation;

/// Data and signal normalization.
pub mod normalization;

/// Peak detection algorithms.
pub mod peaks;

/// Resampling algorithms.
pub mod resample;

/// Traits for signal processing
pub mod traits;

/// Outlier detection and robust interpolation (for RR/IBI cleaning etc.).
pub mod outliers;

/// Windowing helpers and RR-to-tachogram resampling (for equidistant feature windows).
pub mod windows;

/// Per-event probability and 180 s window repair.
pub mod event_window;

/// Sparse / compressed sensing (ISTA, OMP, measurement matrices).
pub mod sparse_sensing;

/// Deconvolution (NNLS hormone path, Wiener).
pub mod deconvolution;

pub use decimate::min_max_decimate;
pub use deconvolution::{
    burst_indices,
    convolve,
    exponential_kernel,
    gh_burst_deconvolve,
    gh_deconvolve,
    gh_deconvolve_sparse,
    gh_reconvolve,
    nonnegative_deconvolution,
    nonnegative_deconvolution_on_support,
    sparse_nonnegative_deconvolution,
};
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
    event_measures,
    interpolate_low_probability,
    peer_fit,
    repair_event_window,
    rolling_event_measures,
    score_event_intervals,
};
pub use interpolation::{
    interp_cubic,
    interp_linear,
    interp_spline,
    interp1,
};
pub use normalization::{
    normalize,
    zscore,
};
pub use outliers::{
    FillStrategy,
    OutlierCriterion,
    detect_outliers,
    interpolate_outliers,
    robust_interpolate,
    robust_interpolate_with_times,
};
pub use peaks::{
    Peak,
    PeakFinder,
    PeakFinderBuilder,
};
pub use resample::{
    Resample,
    ResampleMethod,
};
pub use sparse_sensing::{
    IstaConfig,
    SparseRecoveryResult,
    dct_basis,
    effective_sensing,
    ista,
    measure,
    omp,
    random_gaussian_sensing,
    reconstruct_signal,
    row_selection_sensing,
};
pub use traits::PeakDetect;
pub use windows::resample_rr_to_tachogram;

/// Interpolation functions.
pub mod interp {
    pub use super::interpolation::*;
}

/// Normalization functions.
pub mod norm {
    pub use super::normalization::*;
}
