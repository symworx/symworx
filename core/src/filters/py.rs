// core/src/filters/py.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unsafe_op_in_unsafe_fn)]

use pyo3::prelude::*;

// ==========================================================
// FILTERS
// ==========================================================
// Adaptive Filters 
// ----------------------------------------------------------
#[pyfunction(name = "adaptive_mean_filter")]
pub fn py_adaptive_mean_filter(data: Vec<f64>, k: f64) -> PyResult<Vec<f64>> {
    let out = crate::filters::adaptive::basic::adaptive_mean_filter(&data, k);
    Ok(out)
}

#[pyfunction(name = "adaptive_median_filter")]
pub fn py_adaptive_median_filter(data: Vec<f64>, k: f64) -> PyResult<Vec<f64>> {
    let out = crate::filters::adaptive::basic::adaptive_median_filter(&data, k);
    Ok(out)
}


// ----------------------------------------------------------
// Linear Filters 
// ----------------------------------------------------------
// --- Bandpass ---------------------------------------------
#[pyclass(name = "BandpassFilter")]
pub struct PyBandpassFilter {
    inner: crate::filters::linear::bandpass::BandpassFilter,
}

#[pymethods]
impl PyBandpassFilter {
    #[new]
    fn new(fs: f64, f_low: f64, f_high: f64, q: f64) -> Self {
        Self {
            inner: crate::filters::linear::bandpass::BandpassFilter::new(
                fs, f_low, f_high, q
            ),
        }
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn process(&mut self, data: Vec<f64>) -> Vec<f64> {
        self.inner.process(&data)
    }
}

// --- Chebyshev --------------------------------------------
#[pyclass(name = "ChebyshevFilter")]
pub struct PyChebyshevFilter {
    inner: crate::filters::linear::chebyshev::ChebyshevFilter,
}

#[pymethods]
impl PyChebyshevFilter {
    #[new]
    fn new(fs: f64, f_low: f64, f_high: f64, ripple_db: f64) -> Self {
        Self { 
            inner: crate::filters::linear::chebyshev::ChebyshevFilter::new(
                fs, f_low, f_high, ripple_db
            ),
        }
    }

    fn reset(&mut self) {
        self.inner.reset()
    }

    fn process_sample(&mut self, x: f64) -> f64 {
        self.inner.process_sample(x)
    }

    fn process(&mut self, data: Vec<f64>) -> Vec<f64> {
        self.inner.process(&data)
    }
}


// ----------------------------------------------------------
// Nonlinear Filters 
// ----------------------------------------------------------
#[pyclass(name = "KalmanFilter")]
pub struct PyKalmanFilter {
    inner: crate::filters::nonlinear::kalman::KalmanFilter,
}

#[pymethods]
impl PyKalmanFilter {
    #[new]
    fn new(dt: f64, process_var: f64, meas_var: f64) -> Self {
        Self {
            inner: crate::filters::nonlinear::kalman::KalmanFilter::new(
                dt, process_var, meas_var,
            )
        }
    }

    fn predict(&mut self) {
        self.inner.predict()
    }

    fn update(&mut self, z: f64) {
        self.inner.update(z)
    }

    fn state(&self) -> (f64, f64) {
        self.inner.state()
    }
}

// ----------------------------------------------------------
// Time Frequency
// ----------------------------------------------------------
// Placeholder

