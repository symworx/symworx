// Copyright (c) 2026 SymWorx. All rights reserved.

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use symworx_loadsym::load::{
    calculate_physiological_load,
    calculate_mechanical_load,
    optimize_load,
};

// ==========================================================
// Mechanical load 
// ==========================================================

#[pyfunction(name = "calculate_mechanical_load")]
pub fn py_calculate_mechanical_load(force_data: Vec<f64>, velocity_data: Vec<f64>) -> f64 {
    if force_data.len() != velocity_data.len() {
        panic!("force_data and velocity_data must have the same length ({} vs {})", force_data.len(), velocity_data.len());
    }
    calculate_mechanical_load(&force_data, &velocity_data)
}

// ==========================================================
// Optimization
// ==========================================================

#[pyfunction(name = "optimize_load")]
pub fn py_optimize_load(parameters: Vec<f64>, data: Vec<f64>) -> PyResult<Vec<f64>> {
    Ok(optimize_load(&parameters, &data))
}

// ==========================================================
// Physiological load 
// ==========================================================

#[pyfunction(name = "calculate_physiological_load")]
pub fn py_calculate_physiological_load(hr_data: Vec<f64>) -> PyResult<f64> {
    Ok(calculate_physiological_load(&hr_data))
}

// ==========================================================
// PYTHON REGISTER
// ==========================================================

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {

    m.add_function(wrap_pyfunction!(py_calculate_mechanical_load, m)?)?;
    m.add_function(wrap_pyfunction!(py_optimize_load, m)?)?;
    m.add_function(wrap_pyfunction!(py_calculate_physiological_load, m)?)?;

    Ok(())
}
