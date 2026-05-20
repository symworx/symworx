// symworx-biosym/src/python.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

//! PyO3 bindings for symworx-biosym.
//! Build with maturin or cargo build --release for Python extension.

use pyo3::prelude::*;
use ndarray::Array1;

use crate::biomechanics::{GaitData, GaitParams};
use crate::cpg::{CpgConfig, SymCpgModel, VanDerPol};

#[pymodule]
fn symworx_biosym(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyGaitParams>()?;
    m.add_class::<PyGaitData>()?;
    m.add_class::<PyVanDerPol>()?;
    m.add_class::<PyCpgConfig>()?;
    m.add_class::<PySymCpgModel>()?;
    Ok(())
}

// ============================================================
// Gait bindings
// ============================================================

#[pyclass(name = "GaitParams")]
#[derive(Clone)]
struct PyGaitParams {
    inner: GaitParams,
}

#[pymethods]
impl PyGaitParams {
    #[new]
    fn new() -> Self {
        Self { inner: GaitParams::new() }
    }

    #[staticmethod]
    fn default() -> Self {
        Self { inner: GaitParams::default() }
    }

    fn with_defaults(&mut self) {
        self.inner = self.inner.clone().with_defaults();
    }

    #[getter]
    fn walking_speed(&self) -> f64 { self.inner.walking_speed }
    #[setter]
    fn set_walking_speed(&mut self, v: f64) { self.inner.walking_speed = v; }

    #[getter]
    fn step_length(&self) -> f64 { self.inner.step_length }
    #[setter]
    fn set_step_length(&mut self, v: f64) { self.inner.step_length = v; }

    // Add more getters/setters as needed

    fn __repr__(&self) -> String {
        format!("GaitParams(walking_speed={:.2}, step_length={:.2})", 
                self.inner.walking_speed, self.inner.step_length)
    }
}

#[pyclass(name = "GaitData")]
struct PyGaitData {
    inner: GaitData,
}

#[pymethods]
impl PyGaitData {
    #[new]
    fn new(fs: f64) -> Self {
        Self { inner: GaitData::new(fs) }
    }

    fn calculate_stride_intervals(&mut self) -> Option<Vec<f64>> {
        self.inner.calculate_stride_intervals().map(|a| a.to_vec())
    }

    fn calculate_stride_length(&mut self, walking_speed: Option<f64>) -> Option<Vec<f64>> {
        self.inner.calculate_stride_length(walking_speed).map(|a| a.to_vec())
    }

    fn calculate_step_length(&mut self) -> Option<Vec<f64>> {
        self.inner.calculate_step_length().map(|a| a.to_vec())
    }

    fn calculate_cadence(&self) -> Option<f64> {
        self.inner.calculate_cadence()
    }

    fn calculate_step_times(&mut self) {
        self.inner.calculate_step_times();
    }

    #[getter]
    fn stride_times(&self) -> Option<Vec<f64>> {
        self.inner.stride_times.as_ref().map(|a| a.to_vec())
    }

    #[setter]
    fn set_stride_times(&mut self, v: Vec<f64>) {
        self.inner.stride_times = Some(Array1::from(v));
    }

    fn __repr__(&self) -> String {
        "GaitData(...)".to_string()
    }
}

// ============================================================
// CPG bindings
// ============================================================

#[pyclass(name = "VanDerPol")]
#[derive(Clone)]
struct PyVanDerPol {
    inner: VanDerPol,
}

#[pymethods]
impl PyVanDerPol {
    #[new]
    fn new(mu: f64, x: f64, v: f64) -> Self {
        Self { inner: VanDerPol::new(mu, x, v) }
    }

    fn derivative(&self, omega: f64, forcing: f64) -> (f64, f64) {
        self.inner.derivative(omega, forcing)
    }
}

#[pyclass(name = "CpgConfig")]
#[derive(Clone)]
struct PyCpgConfig {
    inner: CpgConfig,
}

#[pymethods]
impl PyCpgConfig {
    #[new]
    #[pyo3(signature = ( ))]
    fn new() -> Self {
        Self { inner: CpgConfig::default() }
    }

    // Getters for key fields (add more as needed)
    #[getter]
    fn epsilon(&self) -> f64 { self.inner.epsilon }
}

#[pyclass(name = "SymCpgModel")]
struct PySymCpgModel {
    inner: SymCpgModel,
}

#[pymethods]
impl PySymCpgModel {
    #[new]
    #[pyo3(signature = (config = None))]
    fn new(config: Option<PyCpgConfig>) -> Self {
        let cfg = config.map(|c| c.inner);
        Self { inner: SymCpgModel::new(cfg) }
    }

    fn run(&self, t_start: f64, t_end: f64, dt: f64) -> (Vec<f64>, Vec<Vec<f64>>) {
        let (times, states) = self.inner.run((t_start, t_end), dt);
        let states_vec: Vec<Vec<f64>> = states.into_iter().map(|s| s.to_vec()).collect();
        (times, states_vec)
    }

    fn __repr__(&self) -> String {
        "SymCpgModel(...)".to_string()
    }
}
