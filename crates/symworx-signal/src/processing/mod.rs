// Copyright (c) 2026 SymWorx
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

pub use decimate::min_max_decimate;
pub use interpolation::{interp_cubic, interp_linear, interp_spline, interp1};
pub use normalization::{normalize, zscore};
pub use outliers::{
    FillStrategy, OutlierCriterion, detect_outliers, interpolate_outliers, robust_interpolate,
    robust_interpolate_with_times,
};
pub use peaks::{Peak, PeakFinder, PeakFinderBuilder};
pub use resample::{Resample, ResampleMethod};
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
