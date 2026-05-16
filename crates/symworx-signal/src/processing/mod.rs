// symworx/crates/symworx-signal/src/processing/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

//! # Processing Module
//!
//! Signal processing utilities:
//!  * interpolation
//!  * normalization
//!  * resampling
//!  * feature extraction

pub mod interpolation;
pub mod normalization;
pub mod peaks;
pub mod resample;
pub mod traits;


pub use interpolation::{
    interp_linear,
    interp1,
    interp_cubic,
    interp_spline
};
pub use normalization::{normalize, zscore};
pub use peaks::{Peak, PeakFinder, PeakFinderBuilder};
pub use resample::{ResampleMethod, Resample};
pub use traits::{PeakDetect};

// ==========================================================
// Namespaced exports
// ==========================================================

pub mod interp {
    pub use super::interpolation::*;
}

pub mod norm {
    pub use super::normalization::*;
}
