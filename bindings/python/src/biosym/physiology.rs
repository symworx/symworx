// symworx/bindings/python/src/biosym/physiology.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use symworx_biosym::physiology::analyze_ppg;

#[pyfunction(name = "analyze_ppg")]
pub fn py_analyze_ppg(py: Python) -> PyResult<()> {
    analyze_ppg();

    Ok(())
}

// use symworx_biosym::physiology::{
//     gamma_normalization,
// };

// ==========================================================
// PPG
// ==========================================================

// #[pyfunction(name = "gamma_normalization")]
// pub fn py_gamma_normalization(
//     tidal_volume: f64,
//     t_insp: f64,
//     kappa: f64,
//     gamma_k: f64,
// ) -> PyResult<f64> {
//     Ok(gamma_normalization(tidal_volume, t_insp, kappa, gamma_k))
// }

// ==========================================================
// Respiration
// ==========================================================

// ==========================================================
// Python Register
// ==========================================================

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    
    m.add_function(wrap_pyfunction!(py_analyze_ppg, m)?)?;
    // m.add_function(wrap_pyfunction!(py_gamma_normalization, m)?)?;

    Ok(())
}
