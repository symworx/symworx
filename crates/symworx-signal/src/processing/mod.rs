// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Signal processing utilities.
//!
//! Core tools for interpolation, normalization, resampling, decimation
//! (for visualization), peak detection, and feature extraction.

pub mod decimate;
pub mod interpolation;
pub mod normalization;
pub mod peaks;
pub mod resample;
pub mod traits;


// Interpolation
pub use interpolation::{
    interp1,
    interp_cubic,
    interp_linear,
    interp_spline,
};

// Normalization
pub use normalization::{
    normalize,
    zscore,
};

// Peak detection
pub use peaks::{
    Peak,
    PeakFinder,
    PeakFinderBuilder,
};

// Decimation (for visualization)
pub use decimate::min_max_decimate;

// Resampling
pub use resample::{
    Resample,
    ResampleMethod,
};

// Traits
pub use traits::PeakDetect;


// Namespaced convenience exports
/// Interpolation functions.
pub mod interp {
    pub use super::interpolation::*;
}

/// Normalization functions.
pub mod norm {
    pub use super::normalization::*;
}
