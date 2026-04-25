// core/src/statistics/py.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unsafe_op_in_unsafe_fn)]

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

// -----------------------------------------------------------
// Entropy 
// -----------------------------------------------------------
#[pyfunction(name = "sample_entropy")]
pub fn py_sample_entropy(data: Vec<f64>, m: usize, r: f64) -> f64 {
    crate::dynamics::sample_entropy(&data, m, r)
}

