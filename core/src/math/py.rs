// core/src/math/py.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use rand::Rng;

use crate::math::{
    gamma_shape,
    cumtrapz,
    trapz,
    normal_sample,
};

use rand::thread_rng;

// ----------------------------------------------------------
// Gamma functions
// ----------------------------------------------------------
#[pyfunction(name = "gamma_shape")]
pub fn py_gamma_shape(k: f64, theta: f64) -> PyResult<f64> {
    Ok(gamma_shape(k, theta))
}

// ----------------------------------------------------------
// Integration
// ----------------------------------------------------------
#[pyfunction(name = "cumtrapz")]
pub fn py_cumtrapz(x: Vec<f64>, y: Vec<f64>) -> PyResult<Vec<f64>> {
    if x.len() != y.len() {
        return Err(PyValueError::new_err("x and y must have same length"));
    }
    let dx = x[1] - x[0];
    Ok(cumtrapz(&y, dx))
}

#[pyfunction(name = "trapz")]
pub fn py_trapz(x: Vec<f64>, y: Vec<f64>) -> PyResult<f64> {
    if x.len() != y.len() {
        return Err(PyValueError::new_err("x and y must have same length"));
    }
    let dx = x[1] - x[0];
    Ok(trapz(&y, dx))
}

// ----------------------------------------------------------
// Random
// ----------------------------------------------------------
#[pyfunction(name = "normal_sample")]
pub fn py_normal_sample(mean: f64, std: f64) -> PyResult<f64> {
    if std < 0.0 {
        return Err(PyValueError::new_err("std must be non-negative"));
    }
    let mut rng = thread_rng();
    Ok(normal_sample(&mut rng, mean, std))
}
