// core/src/math/py.rs
// Copyright (C) 2026 cSYMd

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use rand::thread_rng;

use crate::math::{
    // Distributions
    beta_kernel,
    beta_pdf,
    gamma_kernel,
    gamma_pdf,
    // Integration
    cumtrapz,
    trapz,
    // Random 
    random::sample,
};

// ==========================================================
// Distributions
// ==========================================================

#[pyfunction(name = "beta_kernel")]
pub fn py_beta_kernel(x: f64, a: f64, b: f64) -> PyResult<f64> {
    Ok(beta_kernel(x, a, b))
}

#[pyfunction(name = "beta_pdf")]
pub fn py_beta_pdf(x: f64, a: f64, b: f64) -> PyResult<f64> {
    Ok(beta_pdf(x, a, b))
}

#[pyfunction(name = "beta_sample")]
pub fn py_beta_sample(a: f64, b: f64) -> PyResult<f64> {
    // Source from random::sample::beta
    if a <= 0.0 || b <= 0.0 {
        return Err(PyValueError::new_err("a and b must be positive"));
    }
    let mut rng = thread_rng();
    Ok(sample::beta(&mut rng, a, b))
}

#[pyfunction(name = "gamma_kernel")]
pub fn py_gamma_kernel(x: f64, shape: f64, rate: f64) -> PyResult<f64> {
    Ok(gamma_kernel(x, shape, rate))
}

#[pyfunction(name = "gamma_pdf")]
pub fn py_gamma_pdf(x: f64, shape: f64, rate: f64) -> PyResult<f64> {
    Ok(gamma_pdf(x, shape, rate))
}

#[pyfunction(name = "gamma_sample")]
pub fn py_gamma_sample(shape: f64, rate: f64) -> PyResult<f64> {
    // Source from random::sample::gamma
    if shape <= 0.0 || rate <= 0.0 {
        return Err(PyValueError::new_err("shape and rate must be positive"));
    }
    let mut rng = thread_rng();
    Ok(sample::gamma(&mut rng, shape, rate))
}

#[pyfunction(name = "normal_sample")]
pub fn py_normal_sample(mean: f64, std: f64) -> PyResult<f64> {
    // Source from random::sample::normal
    if std < 0.0 {
        return Err(PyValueError::new_err("std must be non-negative"));
    }
    let mut rng = thread_rng();
    Ok(sample::normal(&mut rng, mean, std))
}

// ==========================================================
// Integration
// ==========================================================

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
