// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Signal processing utilities.
//!
//! Core tools for interpolation, normalization, resampling, decimation
//! (for visualization), peak detection, and feature extraction.

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

pub use decimate::min_max_decimate;
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
pub use peaks::{
    Peak,
    PeakFinder,
    PeakFinderBuilder,
};
pub use resample::{
    Resample,
    ResampleMethod,
};
pub use traits::PeakDetect;

/// Interpolation functions.
pub mod interp {
    pub use super::interpolation::*;
}

/// Normalization functions.
pub mod norm {
    pub use super::normalization::*;
}
