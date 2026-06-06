// Copyright (c) 2026 SymWorx. All rights reserved.

use pyo3::prelude::*;
use symworx_biosym::biosystems::cpg::{
    CpgConfig,
    SymCpgModel,
};
use symworx_core::math::oscillators::VanDerPol;

#[pyclass(name = "VanDerPol")]
#[derive(Clone)]
pub struct PyVanDerPol {
    inner: VanDerPol,
}

#[pymethods]
impl PyVanDerPol {
    #[new]
    fn new(mu: f64, x: f64, v: f64) -> Self {
        Self {
            inner: VanDerPol::new(mu, x, v),
        }
    }

    fn derivative(&self, omega: f64, forcing: f64) -> (f64, f64) {
        self.inner.derivative(omega, forcing)
    }
}

#[pyclass(name = "CpgConfig")]
#[derive(Clone)]
pub struct PyCpgConfig {
    inner: CpgConfig,
}

#[pymethods]
impl PyCpgConfig {
    #[new]
    #[pyo3(signature = ( ))]
    fn new() -> Self {
        Self {
            inner: CpgConfig::default(),
        }
    }

    #[getter]
    fn epsilon(&self) -> f64 {
        self.inner.epsilon
    }
}

#[pyclass(name = "SymCpgModel")]
pub struct PySymCpgModel {
    inner: SymCpgModel,
}

#[pymethods]
impl PySymCpgModel {
    #[new]
    #[pyo3(signature = (config = None))]
    fn new(config: Option<PyCpgConfig>) -> Self {
        let cfg = config.map(|c| c.inner);
        Self {
            inner: SymCpgModel::new(cfg),
        }
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

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyVanDerPol>()?;
    m.add_class::<PyCpgConfig>()?;
    m.add_class::<PySymCpgModel>()?;

    Ok(())
}
