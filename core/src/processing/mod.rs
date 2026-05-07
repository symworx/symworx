// processing/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

// ==========================================================
// Modules
// ==========================================================
pub mod interpolation;
pub mod normalization;
pub mod resample;
pub mod py;

// ==========================================================
// Exports
// ==========================================================
pub use interpolation::{interp_linear, interp1, interp_cubic, interp_spline};
pub use normalization::{normalize, zscore};
pub use resample::{ResampleMethod, Resample};

pub use py::{
    // --- interpolation ---
    py_interp_linear,
    py_interp1,
    py_interp_cubic,
    py_interp_spline,
    // --- normalization ---
    py_normalize,
    py_zscore,
    // --- resample ---
    PyResampleMethod,
    PyResample,
};

// ==========================================================
// PYTHON REGISTER
// ==========================================================
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
