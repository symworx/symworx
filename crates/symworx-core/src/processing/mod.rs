// symworx/crates/core/processing/mod.rs
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
pub mod resample;


pub use interpolation::{
    interp_linear,
    interp1,
    interp_cubic,
    interp_spline
};
pub use normalization::{normalize, zscore};
pub use resample::{ResampleMethod, Resample};

// ==========================================================
// Namespaced exports
// ==========================================================

pub mod interp {
    pub use super::interpolation::*;
}

pub mod norm {
    pub use super::normalization::*;
}
