// core/dynamics/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

// ==========================================================
// MODULES
// ==========================================================
pub mod embedding;
pub mod entropy;
pub mod rqa;
pub mod py;

// ==========================================================
// EXPORTS
// ==========================================================
pub use embedding::{
    edim,
    fnn
};
pub use entropy::{
    sample_entropy
};
pub use py::{
    py_edim,
    py_sample_entropy
};

// ==========================================================
// PYTHON REGISTER
// ==========================================================
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {

    // --- embedding ---
    // m.add_function(wrap_pyfunction!(py_edim, m)?)?;
    
    // --- entropy ---
    m.add_function(wrap_pyfunction!(py_sample_entropy, m)?)?;

    // --- rqa ---
    // m.add_function(wrap_pyfunction!(py_linreg_l1, m)?)?;

    Ok(())
}
