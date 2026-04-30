// core/src/statistics/py.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;

// ===========================================================
// Dynamics
// ===========================================================
// Embedding 
// -----------------------------------------------------------
#[pyfunction(name = "edim")]
pub fn py_edim(data: Vec<f64>, m: usize, tau: usize) -> Vec<Vec<f64>> {
    crate::dynamics::edim(&data, m, tau)
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
    let result = crate::dynamics::fnn(&data, m, tau, rtol, atol, theiler);
    Ok((result.m, result.fnn_ratio))
}

// -----------------------------------------------------------
// Entropy 
// -----------------------------------------------------------
#[pyfunction(name = "sample_entropy")]
pub fn py_sample_entropy(data: Vec<f64>, m: usize, r: f64) -> f64 {
    crate::dynamics::sample_entropy(&data, m, r)
}

