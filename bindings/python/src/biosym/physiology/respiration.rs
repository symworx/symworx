// Copyright (c) 2026 SymWorx. All rights reserved.

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use symworx_biosym::physiology::{
    RespSimulationParams,
    RespTimeSeries,
    generate_respiration_timeseries,
};

#[pyclass(name = "RespTimeSeries")]
#[derive(Clone)]
pub struct PyRespTimeSeries {
    #[pyo3(get)]
    pub times: Vec<f64>,
    #[pyo3(get)]
    pub flow: Vec<f64>,
    #[pyo3(get)]
    pub volume: Vec<f64>,
    #[pyo3(get)]
    pub inhalation_peaks: Vec<usize>,
    #[pyo3(get)]
    pub exhalation_peaks: Vec<usize>,
}

#[pymethods]
impl PyRespTimeSeries {
    fn __repr__(&self) -> String {
        format!(
            "RespTimeSeries(len={}, inh_peaks={}, exh_peaks={})",
            self.times.len(),
            self.inhalation_peaks.len(),
            self.exhalation_peaks.len()
        )
    }
}

impl From<RespTimeSeries> for PyRespTimeSeries {
    fn from(ts: RespTimeSeries) -> Self {
        Self {
            times: ts.times,
            flow: ts.flow,
            volume: ts.volume,
            inhalation_peaks: ts.inhalation_peaks,
            exhalation_peaks: ts.exhalation_peaks,
        }
    }
}

#[pyclass(name = "RespSimulationParams")]
#[derive(Clone)]
pub struct PyRespSimulationParams {
    #[pyo3(get, set)]
    pub brpm: f64,
    #[pyo3(get, set)]
    pub dur_min: f64,
    #[pyo3(get, set)]
    pub fs: f64,
    #[pyo3(get, set)]
    pub tidal_volume: f64,
    #[pyo3(get, set)]
    pub insp_exp_ratio: f64,
    #[pyo3(get, set)]
    pub kappa_insp: f64,
    #[pyo3(get, set)]
    pub tau_exp: f64,
    #[pyo3(get, set)]
    pub amplitude: f64,
    #[pyo3(get, set)]
    pub noise_level: f64,
    #[pyo3(get, set)]
    pub seed: Option<u64>,
}

#[pymethods]
impl PyRespSimulationParams {
    #[new]
    fn new() -> Self {
        Self::default()
    }
}

impl Default for PyRespSimulationParams {
    fn default() -> Self {
        let p = RespSimulationParams::default();
        p.into()
    }
}

impl From<RespSimulationParams> for PyRespSimulationParams {
    fn from(p: RespSimulationParams) -> Self {
        Self {
            brpm: p.brpm,
            dur_min: p.dur_min,
            fs: p.fs,
            tidal_volume: p.tidal_volume,
            insp_exp_ratio: p.insp_exp_ratio,
            kappa_insp: p.kappa_insp,
            tau_exp: p.tau_exp,
            amplitude: p.amplitude,
            noise_level: p.noise_level,
            seed: p.seed,
        }
    }
}

impl From<PyRespSimulationParams> for RespSimulationParams {
    fn from(py: PyRespSimulationParams) -> Self {
        Self {
            brpm: py.brpm,
            dur_min: py.dur_min,
            fs: py.fs,
            tidal_volume: py.tidal_volume,
            insp_exp_ratio: py.insp_exp_ratio,
            kappa_insp: py.kappa_insp,
            tau_exp: py.tau_exp,
            amplitude: py.amplitude,
            noise_level: py.noise_level,
            seed: py.seed,
        }
    }
}

#[pyfunction(name = "generate_respiration_timeseries")]
pub fn py_generate_respiration_timeseries(
    params: PyRespSimulationParams,
) -> PyRespTimeSeries {
    let rust_params = RespSimulationParams::from(params);
    let ts = generate_respiration_timeseries(&rust_params);
    PyRespTimeSeries::from(ts)
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_generate_respiration_timeseries, m)?)?;

    m.add_class::<PyRespTimeSeries>()?;
    m.add_class::<PyRespSimulationParams>()?;

    Ok(())
}