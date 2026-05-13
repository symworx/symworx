// biosym/src/physiology/py.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;

// ==========================================================
// PPG
// ==========================================================

#[pyfunction]
pub fn py_gamma_normalization(
    tidal_volume: f64,
    t_insp: f64,
    kappa: f64,
    gamma_k: f64,
) -> PyResult<f64> {
    Ok(gamma_normalization(tidal_volume, t_insp, kappa, gamma_k))
}

// ==========================================================
// Respiration
// ==========================================================
