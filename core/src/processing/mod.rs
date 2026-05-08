// processing/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

//! # Processing Module Gateway
//!
//! This module serves as the primary public interface for all signal processing
//! utilities within the `processing` crate. It aggregates functionality from
//! submodules such as interpolation, normalization, and resampling.
//!
//! By importing items from this module, users gain access to the entire,
//! curated API without needing to know the internal structure of the submodules.
//!
//! ## Contents
//!
//! *   **Interpolation:** Various mathematical methods for estimating values
//!     between known data points (e.g., linear, cubic, spline).
//! *   **Normalization:** Utilities for scaling or standardizing data vectors.
//! *   **Resampling:** Tools for changing the sampling rate or structure of signals.
//! *   **Python Bindings:** Exposed functions and classes are ready for direct use
//!     within a Python environment via the `register` function.

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

// ==========================================================
// Modules
// ==========================================================

pub mod interpolation;
pub mod normalization;
pub mod resample;
pub mod py;
// [TODO]: Add Rstats 

// ==========================================================
// Namespaced re-exports
// ==========================================================

/// Interpolation methods (e.g. linear, cubic, spline).
pub mod interp {
    pub use super::interpolation::*;
}

/// Normalization and standardization methods.
pub mod norm {
    pub use super::normalization::*;
}

// ==========================================================
// Top level re-exports
// ==========================================================

// Interpolation
pub use interpolation::{
    interp_linear,
    interp1,
    interp_cubic,
    interp_spline
};

// Normalization
pub use normalization::{
    normalize,
    zscore
};

// Resample
pub use resample::{
    ResampleMethod,
    Resample
};

// ==========================================================
// PYTHON REGISTER
// ==========================================================

pub use py::{
    // --- interpolation ------
    py_interp_linear,
    py_interp1,
    py_interp_cubic,
    py_interp_spline,
    // --- normalization ------
    py_normalize,
    py_zscore,
    // --- resample -----------
    PyResampleMethod,
    PyResample,
};

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // --- Interpolation ------------------------------------
    m.add_function(wrap_pyfunction!(py_interp_linear, m)?)?;
    m.add_function(wrap_pyfunction!(py_interp1, m)?)?;
    m.add_function(wrap_pyfunction!(py_interp_cubic, m)?)?;
    m.add_function(wrap_pyfunction!(py_interp_spline, m)?)?;
    // --- Normalization ------------------------------------
    m.add_function(wrap_pyfunction!(py_normalize, m)?)?;
    m.add_function(wrap_pyfunction!(py_zscore, m)?)?;
    // --- Resample -----------------------------------------
    m.add_class::<PyResampleMethod>()?;
    m.add_class::<PyResample>()?;

    Ok(())
}
