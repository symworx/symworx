// math/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

// ==========================================================
// Modules
// ==========================================================
pub mod gamma;
pub mod integration;
pub mod random;
pub mod py;

// ==========================================================
// Exports
// ==========================================================
pub use gamma::{gamma_shape,};
pub use integration::{cumtrapz, trapz,};
pub use random::normal_sample; 

pub use py::{
    // --- gamma ---
    py_gamma_shape,
    
    // --- integration ---
    py_cumtrapz,
    py_trapz,

    // --- random ---
    py_normal_sample,
};

// ==========================================================
// PYTHON REGISTER
// ==========================================================
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {

    // --- gamma --------------------------------------------
    m.add_function(wrap_pyfunction!(py_gamma_shape, m)?)?;

    // --- integration --------------------------------------
    m.add_function(wrap_pyfunction!(py_cumtrapz, m)?)?;
    m.add_function(wrap_pyfunction!(py_trapz, m)?)?;
    
    // --- random -------------------------------------------
    m.add_function(wrap_pyfunction!(py_normal_sample, m)?)?;

    Ok(())
}
