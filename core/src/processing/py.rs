// core/src/processing/py.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;

// ===========================================================
// Normalization
// ===========================================================
// --- Normalize ---------------------------------------------
#[pyfunction(name = "normalize")]
pub fn py_normalize(data: Vec<f64>) -> Vec<f64> {
    crate::processing::normalize(&data)
}

// --- Z-score -----------------------------------------------
#[pyfunction(name = "zscore")]
pub fn py_zscore(data: Vec<f64>) -> Vec<f64> {
    crate::processing::zscore(&data)
}
