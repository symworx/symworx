// core/src/statistics/py.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unsafe_op_in_unsafe_fn)]

use ndarray::{Array1, Array2};
use numpy::{PyArray2, IntoPyArray};
use pyo3::prelude::*;

// ===========================================================
// Statistics
// ===========================================================
// --- Autocorrelation (acf) ---------------------------------
#[pyfunction(name = "acf")]
pub fn py_acf(signal: Vec<f64>, unbiased: bool) -> Vec<f64> {
    crate::statistics::acf(&signal, unbiased)
}

// --- Basic statistics --------------------------------------
#[pyfunction(name = "mean")]
pub fn py_mean(data: Vec<f64>) -> f64 {
    crate::statistics::mean(&data)
}

#[pyfunction(name = "median")]
pub fn py_median(data: Vec<f64>) -> f64 {
    crate::statistics::median(&data)
}

// --- Correlation --------------------------------------
#[pyfunction(name = "pearson_correlation")]
pub fn py_pearson_correlation<'py>(
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

// --- Linreg --------------------------------------
#[pyfunction]
pub fn py_l1(x: Vec<Vec<f64>>, y: Vec<f64>) -> PyResult<Vec<f64>> {
    let x_arr = Array2::from_shape_vec(
        (x.len(), x[0].len()),
        x.into_iter().flatten().collect()).unwrap();
    let y_arr = Array1::from_vec(y);
    let beta: Array1<f64> = crate::statistics::l1(&x_arr, &y_arr);

    Ok(beta.to_vec())
}

#[pyfunction]
pub fn py_l2(x: Vec<Vec<f64>>, y: Vec<f64>) -> PyResult<Vec<f64>> {
    let x_arr = Array2::from_shape_vec(
        (x.len(), x[0].len()),
        x.into_iter().flatten().collect()).unwrap();
    let y_arr = Array1::from_vec(y);
    let beta: Array1<f64> = crate::statistics::l2(&x_arr, &y_arr);

    Ok(beta.to_vec())
}
