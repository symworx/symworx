// fitlers/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

// ==========================================================
// MODULES
// ==========================================================
pub mod adaptive;
pub mod linear;
pub mod nonlinear;
pub mod py;

// ==========================================================
// EXPORTS
// ==========================================================
pub use adaptive::{adaptive_mean_filter, adaptive_median_filter,};
pub use linear::{BandpassFilter, ChebyshevFilter,};
pub use nonlinear::{KalmanFilter,};
pub use py::{py_adaptive_mean_filter,
             py_adaptive_median_filter,
             PyBandpassFilter,
             PyChebyshevFilter,
             PyKalmanFilter,
};

// ==========================================================
// PYTHON REGISTER
// ==========================================================

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {

    // --- Adaptive -----------------------------------------
    m.add_function(wrap_pyfunction!(py_adaptive_mean_filter, m)?)?;
    m.add_function(wrap_pyfunction!(py_adaptive_median_filter, m)?)?;

    // -- Linear --------------------------------------------
    m.add_class::<PyBandpassFilter>()?;
    m.add_class::<PyChebyshevFilter>()?;

    // -- Nonlinear -----------------------------------------
    m.add_class::<PyKalmanFilter>()?;

    Ok(())
}

