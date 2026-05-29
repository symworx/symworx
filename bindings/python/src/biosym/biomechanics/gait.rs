// Copyright (c) 2026 SymWorx. All rights reserved.

use ndarray::Array1;
use pyo3::prelude::*;
use symworx_biosym::biomechanics::{GaitData, GaitParams};

#[pyclass(name = "GaitParams")]
#[derive(Clone)]
pub struct PyGaitParams {
    inner: GaitParams,
}

#[pymethods]
impl PyGaitParams {
    #[new]
    fn new() -> Self {
        Self {
            inner: GaitParams::new(),
        }
    }

    #[staticmethod]
    fn default() -> Self {
        Self {
            inner: GaitParams::default(),
        }
    }

    fn with_defaults(&mut self) {
        self.inner = self.inner.clone().with_defaults();
    }

    #[getter]
    fn walking_speed(&self) -> f64 {
        self.inner.walking_speed
    }

    #[setter]
    fn set_walking_speed(&mut self, v: f64) {
        self.inner.walking_speed = v;
    }
}

#[pyclass(name = "GaitData")]
#[derive(Clone)]
pub struct PyGaitData {
    inner: GaitData,
}

#[pymethods]
impl PyGaitData {
    #[new]
    fn new(fs: f64) -> Self {
        Self {
            inner: GaitData::new(fs),
        }
    }

    #[getter]
    fn fs(&self) -> f64 {
        self.inner.fs
    }

    #[getter]
    fn stride_times(&self) -> Option<Vec<f64>> {
        self.inner.stride_times.as_ref().map(|a| a.to_vec())
    }

    #[setter]
    fn set_stride_times(&mut self, v: Vec<f64>) {
        self.inner.stride_times = Some(Array1::from(v));
    }

    #[getter]
    fn left_step_times(&self) -> Option<Vec<f64>> {
        self.inner.left_step_times.as_ref().map(|a| a.to_vec())
    }

    #[setter]
    fn set_left_step_times(&mut self, v: Vec<f64>) {
        self.inner.left_step_times = Some(Array1::from(v));
    }

    #[getter]
    fn right_step_times(&self) -> Option<Vec<f64>> {
        self.inner.right_step_times.as_ref().map(|a| a.to_vec())
    }

    #[setter]
    fn set_right_step_times(&mut self, v: Vec<f64>) {
        self.inner.right_step_times = Some(Array1::from(v));
    }

    fn __repr__(&self) -> String {
        "GaitData(...)".to_string()
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyGaitParams>()?;
    m.add_class::<PyGaitData>()?;

    Ok(())
}
