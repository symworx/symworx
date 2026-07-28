// Copyright (c) 2026 SymWorx

use ndarray::Array1;
use pyo3::{
    prelude::*,
    wrap_pyfunction,
};
use symworx_biosym::biomechanics::{
    GaitAnalysis,
    GaitData,
    GaitParams,
    GaitStats,
};

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

    #[getter]
    fn height(&self) -> f64 {
        self.inner.height
    }

    #[setter]
    fn set_height(&mut self, v: f64) {
        self.inner.height = v;
    }

    #[getter]
    fn step_length(&self) -> f64 {
        self.inner.step_length
    }

    #[setter]
    fn set_step_length(&mut self, v: f64) {
        self.inner.step_length = v;
    }

    #[getter]
    fn mass(&self) -> f64 {
        self.inner.mass
    }

    #[setter]
    fn set_mass(&mut self, v: f64) {
        self.inner.mass = v;
    }
}

#[pyclass(name = "GaitStats")]
#[derive(Clone)]
pub struct PyGaitStats {
    #[pyo3(get)]
    pub n_strides: usize,
    #[pyo3(get)]
    pub mean_stride_time_s: f64,
    #[pyo3(get)]
    pub std_stride_time_s: f64,
    #[pyo3(get)]
    pub cadence_steps_min: Option<f64>,
    #[pyo3(get)]
    pub mean_stride_length_m: Option<f64>,
    #[pyo3(get)]
    pub std_stride_length_m: Option<f64>,
    #[pyo3(get)]
    pub mean_step_length_m: Option<f64>,
    #[pyo3(get)]
    pub mean_vertical_oscillation_m: Option<f64>,
    #[pyo3(get)]
    pub gait_speed_ms: Option<f64>,
    #[pyo3(get)]
    pub symmetry: Option<f64>,
}

impl From<GaitStats> for PyGaitStats {
    fn from(s: GaitStats) -> Self {
        Self {
            n_strides: s.n_strides,
            mean_stride_time_s: s.mean_stride_time_s,
            std_stride_time_s: s.std_stride_time_s,
            cadence_steps_min: s.cadence_steps_min,
            mean_stride_length_m: s.mean_stride_length_m,
            std_stride_length_m: s.std_stride_length_m,
            mean_step_length_m: s.mean_step_length_m,
            mean_vertical_oscillation_m: s.mean_vertical_oscillation_m,
            gait_speed_ms: s.gait_speed_ms,
            symmetry: s.symmetry,
        }
    }
}

#[pyclass(name = "GaitAnalysis")]
#[derive(Clone)]
pub struct PyGaitAnalysis {
    #[pyo3(get)]
    pub stats: PyGaitStats,
    #[pyo3(get)]
    pub peak_times: Vec<f64>,
    #[pyo3(get)]
    pub intervals_sec: Vec<f64>,
}

impl From<GaitAnalysis> for PyGaitAnalysis {
    fn from(a: GaitAnalysis) -> Self {
        Self {
            stats: PyGaitStats::from(a.stats),
            peak_times: a.intervals.peak_times,
            intervals_sec: a.intervals.intervals_sec,
        }
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
    fn stride_intervals(&self) -> Option<Vec<f64>> {
        self.inner.stride_intervals.as_ref().map(|a| a.to_vec())
    }

    #[getter]
    fn stride_length(&self) -> Option<Vec<f64>> {
        self.inner.stride_length.as_ref().map(|a| a.to_vec())
    }

    #[getter]
    fn step_length(&self) -> Option<Vec<f64>> {
        self.inner.step_length.as_ref().map(|a| a.to_vec())
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

    #[getter]
    fn pelvis_vertical_position(&self) -> Option<Vec<f64>> {
        self.inner.pelvis_vertical_position.as_ref().map(|a| a.to_vec())
    }

    #[setter]
    fn set_pelvis_vertical_position(&mut self, v: Vec<f64>) {
        self.inner.pelvis_vertical_position = Some(Array1::from(v));
    }

    // --- Calculate / analysis methods (mutating to match Rust API) ---

    fn calculate_stride_intervals(&mut self) -> Option<Vec<f64>> {
        self.inner.calculate_stride_intervals().map(|a| a.to_vec())
    }

    #[pyo3(signature = (walking_speed=None))]
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

    fn calculate_vertical_oscillation(&self) -> Option<Vec<f64>> {
        self.inner.calculate_vertical_oscillation().map(|a| a.to_vec())
    }

    fn calculate_step_intervals(&mut self) -> Option<(Vec<f64>, Vec<f64>)> {
        self.inner
            .calculate_step_intervals()
            .map(|(l, r)| (l.to_vec(), r.to_vec()))
    }

    fn calculate_symmetry(&mut self) -> Option<f64> {
        self.inner.calculate_symmetry()
    }

    #[pyo3(signature = (provided_speed=None))]
    fn to_gait_stats(&self, provided_speed: Option<f64>) -> PyGaitStats {
        PyGaitStats::from(self.inner.to_gait_stats(provided_speed))
    }

    fn __repr__(&self) -> String {
        "GaitData(...)".to_string()
    }
}

// --- Top level analysis functions (mirror physiology) ---

#[pyfunction(name = "detect_gait_strides")]
pub fn py_detect_gait_strides(signal: Vec<f64>, fs: f64) -> PyGaitAnalysis {
    let intervals = symworx_biosym::biomechanics::gait::detect_gait_strides(&signal, fs);
    let analysis = symworx_biosym::biomechanics::gait::analyze_gait_from_times(&intervals.peak_times, None);
    PyGaitAnalysis::from(analysis)
}

#[pyfunction(name = "analyze_gait_signal")]
pub fn py_analyze_gait_signal(signal: Vec<f64>, fs: f64) -> PyGaitAnalysis {
    let analysis = symworx_biosym::biomechanics::gait::analyze_gait_signal(&signal, fs);
    PyGaitAnalysis::from(analysis)
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyGaitParams>()?;
    m.add_class::<PyGaitData>()?;
    m.add_class::<PyGaitStats>()?;
    m.add_class::<PyGaitAnalysis>()?;

    m.add_function(wrap_pyfunction!(py_detect_gait_strides, m)?)?;
    m.add_function(wrap_pyfunction!(py_analyze_gait_signal, m)?)?;

    Ok(())
}
