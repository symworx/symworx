// Copyright (c) 2026 SymWorx

use pyo3::{
    prelude::*,
    wrap_pyfunction,
};
use symworx_biosym::physiology::{
    RespAnalysis,
    RespSignalQuality,
    RespSimulationParams,
    RespTimeSeries,
    analyze_respiration,
    analyze_respiration_with_quality,
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

#[pyclass(name = "RespAnalysis")]
#[derive(Clone)]
pub struct PyRespAnalysis {
    #[pyo3(get)]
    pub mean: f64,
    #[pyo3(get)]
    pub std_dev: f64,
    #[pyo3(get)]
    pub duration_sec: f64,
    #[pyo3(get)]
    pub mean_brpm: f64,
    #[pyo3(get)]
    pub peak_indices: Vec<usize>,
    #[pyo3(get)]
    pub breath_intervals_sec: Vec<f64>,
    #[pyo3(get)]
    pub insp_intervals_sec: Vec<f64>,
    #[pyo3(get)]
    pub exp_intervals_sec: Vec<f64>,
    #[pyo3(get)]
    pub inhalation_peak_indices: Vec<usize>,
    #[pyo3(get)]
    pub exhalation_peak_indices: Vec<usize>,
    #[pyo3(get)]
    pub insp_peak_intervals_sec: Vec<f64>,
    #[pyo3(get)]
    pub exp_peak_intervals_sec: Vec<f64>,
}

impl From<RespAnalysis> for PyRespAnalysis {
    fn from(a: RespAnalysis) -> Self {
        Self {
            mean: a.summary.mean,
            std_dev: a.summary.std_dev,
            duration_sec: a.summary.duration_sec,
            mean_brpm: a.mean_brpm,
            peak_indices: a.intervals.peak_indices,
            breath_intervals_sec: a.intervals.intervals_sec,
            insp_intervals_sec: a.insp_intervals_sec,
            exp_intervals_sec: a.exp_intervals_sec,
            inhalation_peak_indices: a.phase_peaks.inhalation_peak_indices,
            exhalation_peak_indices: a.phase_peaks.exhalation_peak_indices,
            insp_peak_intervals_sec: a.insp_peak_intervals_sec,
            exp_peak_intervals_sec: a.exp_peak_intervals_sec,
        }
    }
}

fn resp_timeseries_from_py(ts: PyRespTimeSeries) -> RespTimeSeries {
    RespTimeSeries {
        times: ts.times,
        flow: ts.flow,
        volume: ts.volume,
        inhalation_peaks: ts.inhalation_peaks,
        exhalation_peaks: ts.exhalation_peaks,
    }
}

#[pyclass(name = "RespSignalQuality", eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PyRespSignalQuality {
    Reference = 0,
    High = 1,
    Moderate = 2,
    Poor = 3,
}

impl From<PyRespSignalQuality> for RespSignalQuality {
    fn from(q: PyRespSignalQuality) -> Self {
        match q {
            PyRespSignalQuality::Reference => RespSignalQuality::Reference,
            PyRespSignalQuality::High => RespSignalQuality::High,
            PyRespSignalQuality::Moderate => RespSignalQuality::Moderate,
            PyRespSignalQuality::Poor => RespSignalQuality::Poor,
        }
    }
}

#[pyfunction(name = "analyze_respiration")]
pub fn py_analyze_respiration(ts: PyRespTimeSeries) -> PyRespAnalysis {
    PyRespAnalysis::from(analyze_respiration(&resp_timeseries_from_py(ts)))
}

#[pyfunction(name = "analyze_respiration_with_quality")]
pub fn py_analyze_respiration_with_quality(
    ts: PyRespTimeSeries,
    quality: PyRespSignalQuality,
) -> PyRespAnalysis {
    let rust_ts = resp_timeseries_from_py(ts);
    PyRespAnalysis::from(analyze_respiration_with_quality(&rust_ts, quality.into()))
}

#[pyfunction(name = "generate_respiration_timeseries")]
pub fn py_generate_respiration_timeseries(params: PyRespSimulationParams) -> PyRespTimeSeries {
    let rust_params = RespSimulationParams::from(params);
    let ts = generate_respiration_timeseries(&rust_params);
    PyRespTimeSeries::from(ts)
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_generate_respiration_timeseries, m)?)?;
    m.add_function(wrap_pyfunction!(py_analyze_respiration, m)?)?;
    m.add_function(wrap_pyfunction!(py_analyze_respiration_with_quality, m)?)?;

    m.add_class::<PyRespTimeSeries>()?;
    m.add_class::<PyRespSimulationParams>()?;
    m.add_class::<PyRespAnalysis>()?;
    m.add_class::<PyRespSignalQuality>()?;

    Ok(())
}
