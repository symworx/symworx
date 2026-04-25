// core/src/processing/py.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unsafe_op_in_unsafe_fn)]

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
#[pyfunction(name = "z_score")]
pub fn py_z_score(data: Vec<f64>) -> Vec<f64> {
    crate::processing::z_score(&data)
}
