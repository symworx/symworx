// Copyright (c) 2026 SymWorx

use pyo3::{prelude::*, wrap_pyfunction};
use symworx_biosym::physiology::{
    PPGNoiseConfig, PPGSignalQuality, PPGSimulationParams, PPGTimeSeries, PpgAnalysis, analyze_ppg,
    analyze_ppg_with_quality, generate_ppg_timeseries, generate_ppg_waveform,
};

// ==========================================================
// PPG
// ==========================================================

#[pyclass(name = "PpgAnalysis")]
#[derive(Clone)]
pub struct PyPpgAnalysis {
    #[pyo3(get)]
    pub mean: f64,
    #[pyo3(get)]
    pub std_dev: f64,
    #[pyo3(get)]
    pub duration_sec: f64,
    #[pyo3(get)]
    pub mean_hr_bpm: f64,
    #[pyo3(get)]
    pub peak_indices: Vec<usize>,
    #[pyo3(get)]
    pub rr_intervals_sec: Vec<f64>,
    #[pyo3(get)]
    pub hrv_rmssd_sec: Option<f64>,
    #[pyo3(get)]
    pub hrv_sdnn_sec: Option<f64>,
}

impl From<PpgAnalysis> for PyPpgAnalysis {
    fn from(a: PpgAnalysis) -> Self {
        Self {
            mean: a.summary.mean,
            std_dev: a.summary.std_dev,
            duration_sec: a.summary.duration_sec,
            mean_hr_bpm: a.mean_hr_bpm,
            peak_indices: a.intervals.peak_indices,
            rr_intervals_sec: a.intervals.intervals_sec,
            hrv_rmssd_sec: a.hrv.rmssd_sec,
            hrv_sdnn_sec: a.hrv.sdnn_sec,
        }
    }
}

fn ppg_timeseries_from_py(ts: PyPPGTimeSeries) -> PPGTimeSeries {
    PPGTimeSeries {
        times: ts.times,
        values: ts.values,
        systolic_peaks: ts.systolic_peaks,
        diastolic_peaks: ts.diastolic_peaks,
    }
}

#[pyfunction(name = "analyze_ppg")]
pub fn py_analyze_ppg(ts: PyPPGTimeSeries) -> PyPpgAnalysis {
    PyPpgAnalysis::from(analyze_ppg(&ppg_timeseries_from_py(ts)))
}

#[pyfunction(name = "analyze_ppg_with_quality")]
pub fn py_analyze_ppg_with_quality(
    ts: PyPPGTimeSeries,
    quality: PyPPGSignalQuality,
) -> PyPpgAnalysis {
    let rust_ts = ppg_timeseries_from_py(ts);
    let quality: PPGSignalQuality = quality.into();
    PyPpgAnalysis::from(analyze_ppg_with_quality(&rust_ts, quality))
}

#[pyclass(name = "PPGTimeSeries")]
#[derive(Clone)]
pub struct PyPPGTimeSeries {
    #[pyo3(get)]
    pub times: Vec<f64>,
    #[pyo3(get)]
    pub values: Vec<f64>,
    #[pyo3(get)]
    pub systolic_peaks: Vec<usize>,
    #[pyo3(get)]
    pub diastolic_peaks: Vec<usize>,
}

#[pymethods]
impl PyPPGTimeSeries {
    fn __repr__(&self) -> String {
        format!(
            "PPGTimeSeries(len={}, peaks_s={}, peaks_d={})",
            self.times.len(),
            self.systolic_peaks.len(),
            self.diastolic_peaks.len()
        )
    }
}

impl From<PPGTimeSeries> for PyPPGTimeSeries {
    fn from(ts: PPGTimeSeries) -> Self {
        Self {
            times: ts.times,
            values: ts.values,
            systolic_peaks: ts.systolic_peaks,
            diastolic_peaks: ts.diastolic_peaks,
        }
    }
}

#[pyfunction(name = "generate_ppg_waveform")]
pub fn py_generate_ppg_waveform(
    t0: f64,
    duration: f64,
    fs: f64,
    params: (f64, f64, f64, f64, f64, f64),
) -> (Vec<f64>, Vec<f64>) {
    generate_ppg_waveform(t0, duration, fs, params)
}

#[pyfunction(name = "generate_ppg_timeseries")]
pub fn py_generate_ppg_timeseries(
    start_time: f64,
    rr_intervals: Vec<f64>,
    count: usize,
    beat_duration: f64,
    fs: f64,
    beat_params: (f64, f64, f64, f64, f64, f64),
    noise_cfg: PyPPGNoiseConfig,
) -> PyPPGTimeSeries {
    let cfg = PPGNoiseConfig::from(noise_cfg);
    let ts = generate_ppg_timeseries(
        start_time,
        &rr_intervals,
        count,
        beat_duration,
        fs,
        beat_params,
        &cfg,
    );
    PyPPGTimeSeries::from(ts)
}

#[pyclass(name = "PPGNoiseConfig")]
#[derive(Clone)]
pub struct PyPPGNoiseConfig {
    #[pyo3(get, set)]
    pub amp_drift_std: f64,
    #[pyo3(get, set)]
    pub mu_drift_std: f64,
    #[pyo3(get, set)]
    pub sigma_drift_std: f64,
    #[pyo3(get, set)]
    pub onset_jitter_std: f64,
    #[pyo3(get, set)]
    pub global_noise_std: f64,
    #[pyo3(get, set)]
    pub smoothing_kernel: usize,
}

#[pymethods]
impl PyPPGNoiseConfig {
    #[new]
    fn new() -> Self {
        Self::default()
    }
}

impl Default for PyPPGNoiseConfig {
    fn default() -> Self {
        let cfg = PPGNoiseConfig::default();
        cfg.into()
    }
}

impl From<PPGNoiseConfig> for PyPPGNoiseConfig {
    fn from(cfg: PPGNoiseConfig) -> Self {
        Self {
            amp_drift_std: cfg.amp_drift_std,
            mu_drift_std: cfg.mu_drift_std,
            sigma_drift_std: cfg.sigma_drift_std,
            onset_jitter_std: cfg.onset_jitter_std,
            global_noise_std: cfg.global_noise_std,
            smoothing_kernel: cfg.smoothing_kernel,
        }
    }
}

impl From<PyPPGNoiseConfig> for PPGNoiseConfig {
    fn from(py_cfg: PyPPGNoiseConfig) -> Self {
        Self {
            amp_drift_std: py_cfg.amp_drift_std,
            mu_drift_std: py_cfg.mu_drift_std,
            sigma_drift_std: py_cfg.sigma_drift_std,
            onset_jitter_std: py_cfg.onset_jitter_std,
            global_noise_std: py_cfg.global_noise_std,
            smoothing_kernel: py_cfg.smoothing_kernel,
        }
    }
}
#[pyclass(name = "PPGSignalQuality", eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PyPPGSignalQuality {
    Reference = 0,
    High = 1,
    Moderate = 2,
    Poor = 3,
    Custom = 4,
}

#[pymethods]
impl PyPPGSignalQuality {
    #[new]
    fn new() -> Self {
        PyPPGSignalQuality::Reference
    }
}

impl From<PyPPGSignalQuality> for PPGSignalQuality {
    fn from(q: PyPPGSignalQuality) -> Self {
        match q {
            PyPPGSignalQuality::Reference => PPGSignalQuality::Reference,
            PyPPGSignalQuality::High => PPGSignalQuality::High,
            PyPPGSignalQuality::Moderate => PPGSignalQuality::Moderate,
            PyPPGSignalQuality::Poor => PPGSignalQuality::Poor,
            PyPPGSignalQuality::Custom => PPGSignalQuality::Custom(PPGNoiseConfig::default()),
        }
    }
}

// ==========================================================
// PPG Simulation Params
// ==========================================================

#[pyclass(name = "PPGSimulationParams")]
#[derive(Clone)]
pub struct PyPPGSimulationParams {
    #[pyo3(get, set)]
    pub fs: f64,
    #[pyo3(get, set)]
    pub duration: f64,
    #[pyo3(get, set)]
    pub beat_params: (f64, f64, f64, f64, f64, f64),
    #[pyo3(get, set)]
    pub noise_config: PyPPGNoiseConfig,
    #[pyo3(get, set)]
    pub seed: Option<u64>,
}

#[pymethods]
impl PyPPGSimulationParams {
    #[new]
    fn new() -> Self {
        Self::default()
    }
}

impl Default for PyPPGSimulationParams {
    fn default() -> Self {
        let p = PPGSimulationParams::default();
        p.into()
    }
}

impl From<PPGSimulationParams> for PyPPGSimulationParams {
    fn from(p: PPGSimulationParams) -> Self {
        Self {
            fs: p.fs,
            duration: p.duration,
            beat_params: p.beat_params,
            noise_config: p.noise_config.into(),
            seed: p.seed,
        }
    }
}

impl From<PyPPGSimulationParams> for PPGSimulationParams {
    fn from(py: PyPPGSimulationParams) -> Self {
        Self {
            fs: py.fs,
            duration: py.duration,
            beat_params: py.beat_params,
            noise_config: py.noise_config.into(),
            seed: py.seed,
        }
    }
}

// ==========================================================
// Python Register
// ==========================================================

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_analyze_ppg, m)?)?;
    m.add_function(wrap_pyfunction!(py_analyze_ppg_with_quality, m)?)?;
    m.add_function(wrap_pyfunction!(py_generate_ppg_waveform, m)?)?;
    m.add_function(wrap_pyfunction!(py_generate_ppg_timeseries, m)?)?;

    m.add_class::<PyPPGTimeSeries>()?;
    m.add_class::<PyPPGNoiseConfig>()?;
    m.add_class::<PyPPGSignalQuality>()?;
    m.add_class::<PyPPGSimulationParams>()?;
    m.add_class::<PyPpgAnalysis>()?;

    Ok(())
}
