// Copyright (c) 2026 SymWorx. All rights reserved.

use ndarray::{Array1, Array2};
use numpy::{PyArray2, IntoPyArray};
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use symworx_core::stats::{
    // autocorrelation
    acf,
    // basic 
    mad,
    mean,
    median,
    percentile,
    // correlation
    correlation_matrix,
    pearson_correlation,
    correlation_matrix_from_vec,
    // distance
    euclidean,
    // errors
    mae,
    mse,
    rmse,
    // linreg
    l1,
    l2,
    // variability
    intervals,
    ibi,
    rmssd,
    sdnn,
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
pub fn py_pearson_correlation(
    data: Vec<Vec<f64>>,
    col1: usize,
    col2: usize
) -> f64 {
    let arr = correlation_matrix_from_vec(&data);
    pearson_correlation(&arr, col1, col2)
}

#[pyfunction(name = "correlation_matrix")]
pub fn py_correlation_matrix<'py>(
    py: Python<'py>,
    data: Vec<Vec<f64>>
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let arr = correlation_matrix_from_vec(&data);

    Ok(arr.into_pyarray(py))
}

#[pyfunction(name = "correlation_matrix_from_vec")]
pub fn py_correlation_matrix_from_vec<'py>(
    py: Python<'py>,
    data: Vec<Vec<f64>>
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
pub fn py_l1(x: Vec<Vec<f64>>, y: Vec<f64>) -> PyResult<Vec<f64>> {
    let x_arr = Array2::from_shape_vec(
        (x.len(), x[0].len()),
        x.into_iter().flatten().collect()).unwrap();
    let y_arr = Array1::from_vec(y);
    let beta: Array1<f64> = l1(&x_arr, &y_arr);

    Ok(beta.to_vec())
}

#[pyfunction(name = "l2")]
pub fn py_l2(x: Vec<Vec<f64>>, y: Vec<f64>) -> PyResult<Vec<f64>> {
    let x_arr = Array2::from_shape_vec(
        (x.len(), x[0].len()),
        x.into_iter().flatten().collect()).unwrap();
    let y_arr = Array1::from_vec(y);
    let beta: Array1<f64> = l2(&x_arr, &y_arr);

    Ok(beta.to_vec())
}

// ==========================================================
// Variabilty
// ==========================================================

#[pyfunction(name = "intervals")]
pub fn py_intervals(data: Vec<f64>) -> Vec<f64> {
    intervals(&data)
}

#[pyfunction(name = "ibi")]
pub fn py_ibi(data: Vec<f64>) -> f64 {
    ibi(&data)
}

#[pyfunction(name = "rmssd")]
pub fn py_rmssd(data: Vec<f64>) -> f64 {
    rmssd(&data)
}

#[pyfunction(name = "sdnn")]
pub fn py_sdnn(data: Vec<f64>) -> f64 {
    sdnn(&data)
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
    m.add_function(wrap_pyfunction!(py_intervals, m)?)?;
    m.add_function(wrap_pyfunction!(py_ibi, m)?)?;
    m.add_function(wrap_pyfunction!(py_rmssd, m)?)?;
    m.add_function(wrap_pyfunction!(py_sdnn, m)?)?;

    Ok(())
}
