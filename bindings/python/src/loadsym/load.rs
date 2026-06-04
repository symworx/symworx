// Copyright (c) 2026 SymWorx. All rights reserved.

use pyo3::{
    prelude::*,
    wrap_pyfunction,
};
use symworx_loadsym::load::{
    AcwrSnapshot,
    RiskLevel,
    calculate_mechanical_load,
    calculate_physiological_load,
    // New high-value surface
    classify_acwr,
    compute_acute_chronic,
    compute_acwr_series,
    compute_ewma_acute_chronic,
    compute_monotony,
    compute_strain,
    optimize_load,
};

// ==========================================================
// Mechanical load
// ==========================================================

#[pyfunction(name = "calculate_mechanical_load")]
pub fn py_calculate_mechanical_load(
    force_data: Vec<f64>,
    velocity_data: Vec<f64>,
) -> PyResult<f64> {
    if force_data.len() != velocity_data.len() {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "force_data and velocity_data must have the same length ({} vs {})",
            force_data.len(),
            velocity_data.len()
        )));
    }
    Ok(calculate_mechanical_load(&force_data, &velocity_data))
}

// ==========================================================
// Optimization
// ==========================================================

#[pyfunction(name = "optimize_load")]
pub fn py_optimize_load(parameters: Vec<f64>, data: Vec<f64>) -> PyResult<Vec<f64>> {
    if parameters.len() != data.len() {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "parameters and data must have the same length ({} vs {})",
            parameters.len(),
            data.len()
        )));
    }
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
// ACWR / Risk (new high-value surface for UNCG + general use)
// ==========================================================

#[pyfunction(name = "compute_acute_chronic")]
pub fn py_compute_acute_chronic(
    daily_loads: Vec<f64>,
    acute_window: usize,
    chronic_window: usize,
) -> PyResult<(f64, f64, f64, String)> {
    match compute_acute_chronic(&daily_loads, acute_window, chronic_window) {
        Ok(s) => Ok((
            s.acute_load,
            s.chronic_load,
            s.acwr,
            s.risk_level.as_str().to_string(),
        )),
        Err(e) => Err(pyo3::exceptions::PyValueError::new_err(e.to_string())),
    }
}

#[pyfunction(name = "compute_ewma_acute_chronic")]
pub fn py_compute_ewma_acute_chronic(
    daily_loads: Vec<f64>,
    acute_span: usize,
    chronic_span: usize,
) -> PyResult<(f64, f64, f64, String)> {
    match compute_ewma_acute_chronic(&daily_loads, acute_span, chronic_span) {
        Ok(s) => Ok((
            s.acute_load,
            s.chronic_load,
            s.acwr,
            s.risk_level.as_str().to_string(),
        )),
        Err(e) => Err(pyo3::exceptions::PyValueError::new_err(e.to_string())),
    }
}

#[pyfunction(name = "classify_acwr")]
pub fn py_classify_acwr(acwr: f64) -> String {
    classify_acwr(acwr).as_str().to_string()
}

#[pyfunction(name = "compute_monotony")]
pub fn py_compute_monotony(daily_loads: Vec<f64>) -> PyResult<f64> {
    compute_monotony(&daily_loads)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

#[pyfunction(name = "compute_strain")]
pub fn py_compute_strain(daily_loads: Vec<f64>) -> PyResult<f64> {
    compute_strain(&daily_loads).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

// ==========================================================
// PYTHON REGISTER
// ==========================================================

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_calculate_mechanical_load, m)?)?;
    m.add_function(wrap_pyfunction!(py_optimize_load, m)?)?;
    m.add_function(wrap_pyfunction!(py_calculate_physiological_load, m)?)?;

    m.add_function(wrap_pyfunction!(py_compute_acute_chronic, m)?)?;
    m.add_function(wrap_pyfunction!(py_compute_ewma_acute_chronic, m)?)?;
    m.add_function(wrap_pyfunction!(py_classify_acwr, m)?)?;
    m.add_function(wrap_pyfunction!(py_compute_monotony, m)?)?;
    m.add_function(wrap_pyfunction!(py_compute_strain, m)?)?;

    Ok(())
}
