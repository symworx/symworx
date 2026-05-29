// Copyright (c) 2026 SymWorx. All rights reserved.

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use symworx_core::signal::filters::{
    adaptive::basic::{adaptive_mean_filter, adaptive_median_filter},
    linear::bandpass::BandpassFilter,
    linear::chebyshev::ChebyshevFilter,
    nonlinear::kalman::KalmanFilter,
};

// ==========================================================
// Adaptive filters
// ==========================================================

#[pyfunction(name = "adaptive_mean_filter")]
pub fn py_adaptive_mean_filter(data: Vec<f64>, k: f64) -> PyResult<Vec<f64>> {
    let out = adaptive_mean_filter(&data, k);
    Ok(out)
}

#[pyfunction(name = "adaptive_median_filter")]
pub fn py_adaptive_median_filter(data: Vec<f64>, k: f64) -> PyResult<Vec<f64>> {
    let out = adaptive_median_filter(&data, k);
    Ok(out)
}

// ==========================================================
// Bandpass filter
// ==========================================================

#[pyclass(name = "BandpassFilter")]
pub struct PyBandpassFilter {
    inner: BandpassFilter,
}

#[pymethods]
impl PyBandpassFilter {
    #[new]
    fn new(fs: f64, f_low: f64, f_high: f64, q: f64) -> Self {
        Self {
            inner: BandpassFilter::new(fs, f_low, f_high, q),
        }
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn process(&mut self, data: Vec<f64>) -> Vec<f64> {
        self.inner.process(&data)
    }
}

// ==========================================================
// Chebyshev filter
// ==========================================================

#[pyclass(name = "ChebyshevFilter")]
pub struct PyChebyshevFilter {
    inner: ChebyshevFilter,
}

#[pymethods]
impl PyChebyshevFilter {
    #[new]
    fn new(fs: f64, f_low: f64, f_high: f64, ripple_db: f64) -> Self {
        Self {
            inner: ChebyshevFilter::new(fs, f_low, f_high, ripple_db),
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

// ==========================================================
// Nonlinear Filters
// ==========================================================
#[pyclass(name = "KalmanFilter")]
pub struct PyKalmanFilter {
    inner: KalmanFilter,
}

#[pymethods]
impl PyKalmanFilter {
    #[new]
    fn new(dt: f64, process_var: f64, meas_var: f64) -> Self {
        Self {
            inner: KalmanFilter::new(dt, process_var, meas_var),
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

// ==========================================================
// Time Frequency
// ==========================================================
// Placeholder

// ==========================================================
// PYTHON REGISTER
// ==========================================================

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_adaptive_mean_filter, m)?)?;
    m.add_function(wrap_pyfunction!(py_adaptive_median_filter, m)?)?;

    m.add_class::<PyBandpassFilter>()?;
    m.add_class::<PyChebyshevFilter>()?;
    m.add_class::<PyKalmanFilter>()?;

    Ok(())
}
