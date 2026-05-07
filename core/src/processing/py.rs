// core/src/processing/py.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;


// ==========================================================
// SIGNAL & DATA PROCESSING
// ==========================================================
// Normalization
// ----------------------------------------------------------
#[pyfunction(name = "normalize")]
pub fn py_normalize(data: Vec<f64>) -> Vec<f64> {
    crate::processing::normalize(&data)
}

#[pyfunction(name = "zscore")]
pub fn py_zscore(data: Vec<f64>) -> Vec<f64> {
    crate::processing::zscore(&data)
}

// ----------------------------------------------------------
// Interpolation
// ----------------------------------------------------------
// ----------------------------------------------------------
// Interpolation (Python bindings)
// ----------------------------------------------------------

#[pyfunction(name = "interp_linear")]
pub fn py_interp_linear(x: Vec<f64>, y: Vec<f64>, x_new: Vec<f64>) -> PyResult<Vec<f64>> {
    Ok(crate::processing::interp_linear(&x, &y, &x_new))
}

#[pyfunction(name = "interp1")]
pub fn py_interp1(x: Vec<f64>, y: Vec<f64>, x_new: Vec<f64>) -> PyResult<Vec<f64>> {
    Ok(crate::processing::interp1(&x, &y, &x_new))
}

#[pyfunction(name = "interp_cubic")]
pub fn py_interp_cubic(x: Vec<f64>, y: Vec<f64>, x_new: Vec<f64>) -> PyResult<Vec<f64>> {
    eprintln!("Warning: cubic interpolation not implemented");
    Ok(crate::processing::interp_cubic(&x, &y, &x_new))
}

#[pyfunction(name = "interp_spline")]
pub fn py_interp_spline(x: Vec<f64>, y: Vec<f64>, x_new: Vec<f64>) -> PyResult<Vec<f64>> {
    eprintln!("Warning: spline interpolation not implemented");
    Ok(crate::processing::interp_spline(&x, &y, &x_new))
}

// ----------------------------------------------------------
// Resmpling
// ----------------------------------------------------------
#[pyclass(name = "ResampleMethod")]
#[derive(Clone)]
pub struct PyResampleMethod {
    pub inner: crate::processing::ResampleMethod,
}

#[pymethods]
impl PyResampleMethod {
    #[classattr]
    pub const LINEAR: Self = Self { inner: crate::processing::ResampleMethod::Linear };

    #[classattr]
    pub const CUBIC: Self = Self { inner: crate::processing::ResampleMethod::Cubic };

    #[classattr]
    pub const SPLINE: Self = Self { inner: crate::processing::ResampleMethod::Spline };
}


#[pyclass(name = "Resample")]
pub struct PyResample {
    inner: crate::processing::Resample<'static>,
    data: Vec<f64>, // owned buffer to keep data alive
}

#[pymethods]
impl PyResample {
    #[new]
    pub fn new(y: Vec<f64>) -> Self {
        let data = y;
        let slice: &'static [f64] =
            unsafe { std::mem::transmute::<&[f64], &'static [f64]>(&data) };

        Self {
            inner: crate::processing::Resample::new(slice),
            data,
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_len(&mut self, n: usize) {
        self.inner.to_len(n);
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn scale(&mut self, factor: f64) {
        self.inner.scale(factor);
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_rate(&mut self, old_fs: f64, new_fs: f64) {
        self.inner.to_rate(old_fs, new_fs);
    }

    pub fn method(&self, method: &PyResampleMethod) -> Vec<f64> {
        self.inner.method(method.inner)
    }
}
