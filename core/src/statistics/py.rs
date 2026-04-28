// core/src/statistics/py.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use ndarray::{Array1, Array2};
use numpy::{PyArray2, IntoPyArray};
use pyo3::prelude::*;

// ===========================================================
// Statistics
// ===========================================================
// Autocorrelation
// -----------------------------------------------------------
#[pyfunction(name = "acf")]
pub fn py_acf(signal: Vec<f64>, unbiased: bool) -> Vec<f64> {
    crate::statistics::acf(&signal, unbiased)
}

// -----------------------------------------------------------
// Basic statistics 
// -----------------------------------------------------------
#[pyfunction(name = "mean")]
pub fn py_mean(data: Vec<f64>) -> f64 {
    crate::statistics::mean(&data)
}

#[pyfunction(name = "median")]
pub fn py_median(data: Vec<f64>) -> f64 {
    crate::statistics::median(&data)
}

#[pyfunction(name = "mad")]
pub fn py_mad(data: Vec<f64>) -> f64 {
    let med = crate::statistics::median(&data);
    crate::statistics::mad(&data, med)
}

#[pyfunction(name = "percentile")]
pub fn py_percentile(data: Vec<f64>, p: Vec<f64>) -> PyResult<Vec<f64>> {
    Ok(crate::statistics::percentile(&data, p))
}

// -----------------------------------------------------------
// Correlation 
// -----------------------------------------------------------
#[pyfunction(name = "pearson_correlation")]
pub fn py_pearson_correlation(
    data: Vec<Vec<f64>>,
    col1: usize,
    col2: usize
) -> f64 {
    let arr = crate::statistics::correlation_matrix_from_vec(&data);
    crate::statistics::pearson_correlation(&arr, col1, col2)
}

#[pyfunction(name = "correlation_matrix")]
pub fn py_correlation_matrix<'py>(
    py: Python<'py>,
    data: Vec<Vec<f64>>
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let arr = crate::statistics::correlation_matrix_from_vec(&data);

    Ok(arr.into_pyarray_bound(py))
}

#[pyfunction(name = "correlation_matrix_from_vec")]
pub fn py_correlation_matrix_from_vec<'py>(
    py: Python<'py>,
    data: Vec<Vec<f64>>
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let arr = crate::statistics::correlation_matrix_from_vec(&data);

    Ok(arr.into_pyarray_bound(py))
}

// -----------------------------------------------------------
// Euclidean distance 
// -----------------------------------------------------------
#[pyfunction(name = "euclidean")]
pub fn py_euclidean(vec1: Vec<f64>, vec2: Vec<f64>) -> f64 {
    crate::statistics::euclidean(&vec1, &vec2)
}

// -----------------------------------------------------------
// Errors
// -----------------------------------------------------------
#[pyfunction(name = "mae")]
pub fn py_mae(actual: Vec<f64>, predicted: Vec<f64>) -> f64 {
    crate::statistics::mae(&actual, &predicted)
}

#[pyfunction(name = "mse")]
pub fn py_mse(actual: Vec<f64>, predicted: Vec<f64>) -> f64 {
    crate::statistics::mse(&actual, &predicted)
}

#[pyfunction(name = "rmse")]
pub fn py_rmse(actual: Vec<f64>, predicted: Vec<f64>) -> f64 {
    crate::statistics::rmse(&actual, &predicted)
}

// -----------------------------------------------------------
// Linear Regression
// -----------------------------------------------------------
#[pyfunction(name = "l1")]
pub fn py_l1(x: Vec<Vec<f64>>, y: Vec<f64>) -> PyResult<Vec<f64>> {
    let x_arr = Array2::from_shape_vec(
        (x.len(), x[0].len()),
        x.into_iter().flatten().collect()).unwrap();
    let y_arr = Array1::from_vec(y);
    let beta: Array1<f64> = crate::statistics::l1(&x_arr, &y_arr);

    Ok(beta.to_vec())
}

#[pyfunction(name = "l2")]
pub fn py_l2(x: Vec<Vec<f64>>, y: Vec<f64>) -> PyResult<Vec<f64>> {
    let x_arr = Array2::from_shape_vec(
        (x.len(), x[0].len()),
        x.into_iter().flatten().collect()).unwrap();
    let y_arr = Array1::from_vec(y);
    let beta: Array1<f64> = crate::statistics::l2(&x_arr, &y_arr);

    Ok(beta.to_vec())
}

// -----------------------------------------------------------
// Variabilty
// -----------------------------------------------------------
#[pyfunction(name = "intervals")]
pub fn py_intervals(data: Vec<f64>) -> Vec<f64> {
    crate::statistics::intervals(&data)
}

#[pyfunction(name = "ibi")]
pub fn py_ibi(data: Vec<f64>) -> f64 {
    crate::statistics::ibi(&data)
}

#[pyfunction(name = "rmssd")]
pub fn py_rmssd(data: Vec<f64>) -> f64 {
    crate::statistics::rmssd(&data)
}

#[pyfunction(name = "sdnn")]
pub fn py_sdnn(data: Vec<f64>) -> f64 {
    crate::statistics::sdnn(&data)
}
