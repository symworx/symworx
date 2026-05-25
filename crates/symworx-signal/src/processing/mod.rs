// Copyright (C) 2026 cSYMd, All rights reserved.

//! Signal processing utilities.
//!
//! Core tools for interpolation, normalization, resampling,
//! peak detection, and feature extraction.

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
