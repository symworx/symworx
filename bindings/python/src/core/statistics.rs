// Copyright (c) 2026 SymWorx. All rights reserved.

use ndarray::{
    Array1,
    Array2,
};
use numpy::{
    IntoPyArray,
    PyArray2,
};
use pyo3::{
    prelude::*,
    wrap_pyfunction,
};
use symworx_core::stats::{
    // autocorrelation
    acf,
    // correlation
    correlation_matrix,
    correlation_matrix_from_vec,
    // distance
    euclidean,
    // linreg
    l1,
    l2,
    // basic
    mad,
    // errors
    mae,
    mean,
    mean_successive_differences,
    median,
    mse,
    pearson_correlation,
    percentile,
    rmse,
    rmssd,
    sd_successive_differences,
    // variability
    successive_differences,
};

// ==========================================================
// Autocorrelation
// ==========================================================

#[pyfunction(name = "acf")]
pub fn py_acf(signal: Vec<f64>, unbiased: bool) -> Vec<f64> {
    acf(&signal, unbiased)
}

// ==========================================================
// Basic statistics
// ==========================================================

#[pyfunction(name = "mean")]
pub fn py_mean(data: Vec<f64>) -> f64 {
    mean(&data)
}

#[pyfunction(name = "median")]
pub fn py_median(data: Vec<f64>) -> f64 {
    median(&data)
}

#[pyfunction(name = "mad")]
pub fn py_mad(data: Vec<f64>) -> f64 {
    let med = median(&data);
    mad(&data, med)
}

#[pyfunction(name = "percentile")]
pub fn py_percentile(data: Vec<f64>, p: Vec<f64>) -> PyResult<Vec<f64>> {
    Ok(percentile(&data, p))
}

// ==========================================================
// Correlation
// ==========================================================

#[pyfunction(name = "pearson_correlation")]
pub fn py_pearson_correlation(data: Vec<Vec<f64>>, col1: usize, col2: usize) -> f64 {
    let arr = correlation_matrix_from_vec(&data);
    pearson_correlation(&arr, col1, col2)
}

#[pyfunction(name = "correlation_matrix")]
pub fn py_correlation_matrix<'py>(
    py: Python<'py>,
    data: Vec<Vec<f64>>,
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let arr = correlation_matrix_from_vec(&data);

    Ok(arr.into_pyarray(py))
}

#[pyfunction(name = "correlation_matrix_from_vec")]
pub fn py_correlation_matrix_from_vec<'py>(
    py: Python<'py>,
    data: Vec<Vec<f64>>,
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let arr = correlation_matrix_from_vec(&data);

    Ok(arr.into_pyarray(py))
}

// ==========================================================
// Euclidean distance
// ==========================================================

#[pyfunction(name = "euclidean")]
pub fn py_euclidean(vec1: Vec<f64>, vec2: Vec<f64>) -> f64 {
    euclidean(&vec1, &vec2)
}

// ==========================================================
// Errors
// ==========================================================

#[pyfunction(name = "mae")]
pub fn py_mae(actual: Vec<f64>, predicted: Vec<f64>) -> f64 {
    mae(&actual, &predicted)
}

#[pyfunction(name = "mse")]
pub fn py_mse(actual: Vec<f64>, predicted: Vec<f64>) -> f64 {
    mse(&actual, &predicted)
}

#[pyfunction(name = "rmse")]
pub fn py_rmse(actual: Vec<f64>, predicted: Vec<f64>) -> f64 {
    rmse(&actual, &predicted)
}

// ==========================================================
// Linear Regression
// ==========================================================

#[pyfunction(name = "l1")]
pub fn py_l1(
    x: Vec<Vec<f64>>,
    y: Vec<f64>,
    alpha: Option<f64>,
    max_iter: Option<usize>,
    tol: Option<f64>,
) -> PyResult<Vec<f64>> {
    // Convert Python lists to ndarray
    let x_arr = Array2::from_shape_vec((x.len(), x[0].len()), x.into_iter().flatten().collect())
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid X shape: {}", e)))?;

    let y_arr = Array1::from_vec(y);

    // Use defaults if not provided
    let alpha = alpha.unwrap_or(0.1);
    let max_iter = max_iter.unwrap_or(200);
    let tol = tol.unwrap_or(1e-6);

    // Call the Rust Lasso implementation
    let beta = l1(&x_arr, &y_arr, alpha, max_iter, tol);

    Ok(beta.to_vec())
}

#[pyfunction(name = "l2")]
pub fn py_l2(x: Vec<Vec<f64>>, y: Vec<f64>) -> PyResult<Vec<f64>> {
    let x_arr = Array2::from_shape_vec((x.len(), x[0].len()), x.into_iter().flatten().collect())
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid X shape: {}", e)))?;

    let y_arr = Array1::from_vec(y);

    let beta = l2(&x_arr, &y_arr);
    Ok(beta.to_vec())
}

// ==========================================================
// Variabilty
// ==========================================================

#[pyfunction(name = "successive_differences")]
pub fn py_successive_differences(data: Vec<f64>) -> Vec<f64> {
    successive_differences(&data)
}

#[pyfunction(name = "mean_successive_differences")]
pub fn py_mean_successive_differences(data: Vec<f64>) -> f64 {
    mean_successive_differences(&data)
}

#[pyfunction(name = "rmssd")]
pub fn py_rmssd(data: Vec<f64>) -> f64 {
    rmssd(&data)
}

#[pyfunction(name = "sd_successive_differences")]
pub fn py_sd_successive_differences(data: Vec<f64>) -> f64 {
    sd_successive_differences(&data)
}

// ==========================================================
// PYTHON REGISTER
// ==========================================================

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // --- Autocorrelation ----------------------------------
    m.add_function(wrap_pyfunction!(py_acf, m)?)?;

    // --- Basic statistics ---------------------------------
    m.add_function(wrap_pyfunction!(py_mean, m)?)?;
    m.add_function(wrap_pyfunction!(py_median, m)?)?;
    m.add_function(wrap_pyfunction!(py_mad, m)?)?;
    m.add_function(wrap_pyfunction!(py_percentile, m)?)?;

    // --- Correlation --------------------------------------
    let _ = m.add_function(wrap_pyfunction!(py_pearson_correlation, m)?);
    m.add_function(wrap_pyfunction!(py_correlation_matrix, m)?)?;
    let _ = m.add_function(wrap_pyfunction!(py_correlation_matrix_from_vec, m)?);

    // --- Distance -----------------------------------------
    m.add_function(wrap_pyfunction!(py_euclidean, m)?)?;

    // --- Errors -------------------------------------------
    m.add_function(wrap_pyfunction!(py_mae, m)?)?;
    m.add_function(wrap_pyfunction!(py_mse, m)?)?;
    m.add_function(wrap_pyfunction!(py_rmse, m)?)?;

    // --- Linear regression --------------------------------
    m.add_function(wrap_pyfunction!(py_l1, m)?)?;
    m.add_function(wrap_pyfunction!(py_l2, m)?)?;

    // --- Variability --------------------------------------
    m.add_function(wrap_pyfunction!(py_successive_differences, m)?)?;
    m.add_function(wrap_pyfunction!(py_mean_successive_differences, m)?)?;
    m.add_function(wrap_pyfunction!(py_rmssd, m)?)?;
    m.add_function(wrap_pyfunction!(py_sd_successive_differences, m)?)?;

    Ok(())
}
