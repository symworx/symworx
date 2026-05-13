// symworx/bindings/python/src/core/dynamics.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use symworx_core::dynamics::{edim, fnn, sample_entropy};

// ================================================
// Python bindings
// ================================================

#[pyfunction(name = "edim")]
pub fn py_edim(data: Vec<f64>, m: usize, tau: usize) -> Vec<Vec<f64>> {
    edim(&data, m, tau)
}

#[pyfunction(name = "fnn")]
pub fn py_fnn(
    data: Vec<f64>,
    m: usize,
    tau: usize,
    rtol: f64,
    atol: f64,
    theiler: usize
) -> PyResult<(usize, f64)> {
    let result = fnn(&data, m, tau, rtol, atol, theiler);
    Ok((result.m, result.fnn_ratio))
}

#[pyfunction(name = "sample_entropy")]
pub fn py_sample_entropy(data: Vec<f64>, m: usize, r: f64) -> f64 {
    sample_entropy(&data, m, r)
}

// ================================================
// Python register
// ================================================

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_edim, m)?)?;
    m.add_function(wrap_pyfunction!(py_fnn, m)?)?;
    m.add_function(wrap_pyfunction!(py_sample_entropy, m)?)?;
    // m.add_function(wrap_pyfunction!(py_linreg_l1, m)?)?;

    Ok(())
}
