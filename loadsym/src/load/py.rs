// core/src/filters/py.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;

// ==========================================================
// LOAD
// ==========================================================
// Mechanical
// ----------------------------------------------------------
#[pyfunction(name = "calculate_mechanical_load")]
pub fn py_calculate_mechanical_load(force_data: Vec<f64>, velocity_data: Vec<f64>) -> f64 {
    if force_data.len() != velocity_data.len() {
        panic!("force_data and velocity_data must have the same length ({} vs {})", force_data.len(), velocity_data.len());
    }
    crate::load::calculate_mechanical_load(&force_data, &velocity_data)
}

// ----------------------------------------------------------
// Optimization
// ----------------------------------------------------------
#[pyfunction(name = "optimize_load")]
pub fn py_optimize_load(parameters: Vec<f64>, data: Vec<f64>) -> PyResult<Vec<f64>> {
    Ok(crate::load::optimize_load(&parameters, &data))
}

// ----------------------------------------------------------
// Optimization
// ----------------------------------------------------------
#[pyfunction(name = "calculate_physiological_load")]
pub fn py_calculate_physiological_load(hr_data: Vec<f64>) -> PyResult<f64> {
    Ok(crate::load::calculate_physiological_load(&hr_data))
}
