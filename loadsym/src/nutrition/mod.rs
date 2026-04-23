// loadsym/src/nutrition/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.
//
#![allow(unused_imports)]

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

pub mod bodycomp;

// re-exports of specific functions
pub use bodycomp::{calculate_bmr, py_calculate_bmr, calculate_tdee, py_calculate_tdee};

// ==========================================================
// PYTHON REGISTER
// ==========================================================
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {

    // --- body composition ---
    m.add_function(wrap_pyfunction!(py_calculate_bmr, m)?)?;
    m.add_function(wrap_pyfunction!(py_calculate_tdee, m)?)?;

    Ok(())
}
