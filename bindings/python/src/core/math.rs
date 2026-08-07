// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

use pyo3::{
    exceptions::PyValueError,
    prelude::*,
    wrap_pyfunction,
};
use rand::rng;
use symworx_core::math::{
    // Distributions
    beta_kernel,
    beta_pdf,
    // Integration
    cumtrapz,
    // Series / sequence operations (new from math migration)
    ewma,
    gamma_kernel,
    gamma_pdf,
    // Random
    random::sample,
    rolling_mean,
    rolling_std,
    successive_absolute_differences,
    successive_differences,
    trapz,
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
    let mut rng = rng();
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
    let mut rng = rng();
    Ok(sample::gamma(&mut rng, shape, rate))
}

#[pyfunction(name = "normal_sample")]
pub fn py_normal_sample(mean: f64, std: f64) -> PyResult<f64> {
    // Source from random::sample::normal
    if std < 0.0 {
        return Err(PyValueError::new_err("std must be non-negative"));
    }
    let mut rng = rng();
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

// ==========================================================
// Series / Sequential Differences (from symworx-math migration)
// ==========================================================

#[pyfunction(name = "successive_differences")]
pub fn py_successive_differences(data: Vec<f64>) -> Vec<f64> {
    successive_differences(&data)
}

#[pyfunction(name = "successive_absolute_differences")]
pub fn py_successive_absolute_differences(data: Vec<f64>) -> Vec<f64> {
    successive_absolute_differences(&data)
}

#[pyfunction(name = "ewma")]
pub fn py_ewma(data: Vec<f64>, span: usize) -> Vec<f64> {
    ewma(&data, span)
}

#[pyfunction(name = "rolling_mean")]
pub fn py_rolling_mean(data: Vec<f64>, window: usize) -> Vec<f64> {
    rolling_mean(&data, window)
}

#[pyfunction(name = "rolling_std")]
pub fn py_rolling_std(data: Vec<f64>, window: usize) -> Vec<f64> {
    rolling_std(&data, window)
}

// ==========================================================
// Python Registration
// ==========================================================

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Distributions
    m.add_function(wrap_pyfunction!(py_beta_kernel, m)?)?;
    m.add_function(wrap_pyfunction!(py_beta_pdf, m)?)?;
    m.add_function(wrap_pyfunction!(py_gamma_kernel, m)?)?;
    m.add_function(wrap_pyfunction!(py_gamma_pdf, m)?)?;

    // Integration
    m.add_function(wrap_pyfunction!(py_cumtrapz, m)?)?;
    m.add_function(wrap_pyfunction!(py_trapz, m)?)?;

    // Sample
    m.add_function(wrap_pyfunction!(py_beta_sample, m)?)?;
    m.add_function(wrap_pyfunction!(py_gamma_sample, m)?)?;
    m.add_function(wrap_pyfunction!(py_normal_sample, m)?)?;

    // Series / Differences (new from symworx-math)
    m.add_function(wrap_pyfunction!(py_successive_differences, m)?)?;
    m.add_function(wrap_pyfunction!(py_successive_absolute_differences, m)?)?;
    m.add_function(wrap_pyfunction!(py_ewma, m)?)?;
    m.add_function(wrap_pyfunction!(py_rolling_mean, m)?)?;
    m.add_function(wrap_pyfunction!(py_rolling_std, m)?)?;

    Ok(())
}
