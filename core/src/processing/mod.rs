// processing/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

// ==========================================================
// Modules
// ==========================================================
pub mod normalization;
pub mod py;

// ==========================================================
// Exports
// ==========================================================
pub use normalization::{normalize, zscore};
pub use py::{py_normalize, py_zscore};

// ==========================================================
// PYTHON REGISTER
// ==========================================================
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {

    // --- Normalization ------------------------------------
    m.add_function(wrap_pyfunction!(py_normalize, m)?)?;
    m.add_function(wrap_pyfunction!(py_zscore, m)?)?;

    Ok(())
}
