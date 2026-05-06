// loadsym/src/load/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

pub mod mechanical;
pub mod optimization;
pub mod physiological;
pub mod py;

// re-exports of specific functions
pub use mechanical::{calculate_mechanical_load};
pub use optimization::{optimize_load};
pub use physiological::{calculate_physiological_load};
pub use py::{
    py_calculate_mechanical_load,
    py_optimize_load,
    py_calculate_physiological_load
};

// ==========================================================
// PYTHON REGISTER
// ==========================================================
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {

    // --- mechanical load ---
    m.add_function(wrap_pyfunction!(py_calculate_mechanical_load, m)?)?;

    // --- optimization ---
    m.add_function(wrap_pyfunction!(py_optimize_load, m)?)?;

    // --- physiological load ---
    m.add_function(wrap_pyfunction!(py_calculate_physiological_load, m)?)?;

    Ok(())
}
