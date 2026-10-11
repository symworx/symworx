// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

use ndarray::Array2;
use numpy::PyArray2;
use pyo3::{
    prelude::*,
    wrap_pyfunction,
};
use symworx_core::dynamics::{
    DEFAULT_AMI_BINS,
    DEFAULT_LMIN,
    DEFAULT_VMIN,
    RecurrencePlot,
    RqaResult,
    TeConfig,
    ami,
    cross_ami,
    crqa,
    edim,
    first_local_minimum,
    fnn,
    multiscale_entropy,
    mutual_information,
    mutual_information_discrete,
    rqa,
    rqa_from_trajectory,
    sample_entropy,
    transfer_entropy,
    transfer_entropy_conditional,
    transfer_entropy_discrete,
    transfer_entropy_discrete_conditional,
    transfer_entropy_discrete_mv,
    transfer_entropy_mv,
    transfer_entropy_with,
};

// ================================================
// Python bindings - RQA support
// ================================================

/// Python-facing RQA result container.
///
/// Mirrors the fields of the Rust `RqaResult`. All ratio measures are in [0, 1].
#[pyclass(name = "RqaResult")]
#[derive(Clone, Debug)]
pub struct PyRqaResult {
    #[pyo3(get)]
    pub recurrence_rate: f64,
    #[pyo3(get)]
    pub determinism: f64,
    #[pyo3(get)]
    pub laminarity: f64,
    #[pyo3(get)]
    pub lmax: usize,
    #[pyo3(get)]
    pub lmean: f64,
    #[pyo3(get)]
    pub lentr: f64,
    #[pyo3(get)]
    pub trapping_time: f64,
    #[pyo3(get)]
    pub vmax: usize,
    #[pyo3(get)]
    pub n_recurrences: usize,
}

#[pymethods]
impl PyRqaResult {
    fn __repr__(&self) -> String {
        format!(
            "RqaResult(RR={:.4}, DET={:.4}, LAM={:.4}, Lmax={}, Lentr={:.3})",
            self.recurrence_rate, self.determinism, self.laminarity, self.lmax, self.lentr
        )
    }

    /// Return a plain Python dict (convenient for pandas, JSON, etc.).
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("recurrence_rate", self.recurrence_rate)?;
        dict.set_item("determinism", self.determinism)?;
        dict.set_item("laminarity", self.laminarity)?;
        dict.set_item("lmax", self.lmax)?;
        dict.set_item("lmean", self.lmean)?;
        dict.set_item("lentr", self.lentr)?;
        dict.set_item("trapping_time", self.trapping_time)?;
        dict.set_item("vmax", self.vmax)?;
        dict.set_item("n_recurrences", self.n_recurrences)?;
        Ok(dict.into())
    }
}

impl From<RqaResult> for PyRqaResult {
    fn from(r: RqaResult) -> Self {
        Self {
            recurrence_rate: r.recurrence_rate,
            determinism: r.determinism,
            laminarity: r.laminarity,
            lmax: r.lmax,
            lmean: r.lmean,
            lentr: r.lentr,
            trapping_time: r.trapping_time,
            vmax: r.vmax,
            n_recurrences: r.n_recurrences,
        }
    }
}

// ================================================
// RecurrencePlot exposure
// ================================================

/// Python wrapper for `RecurrencePlot`.
///
/// The main value is the `.matrix` property, which returns the binary
/// recurrence matrix as a numpy boolean array.
#[pyclass(name = "RecurrencePlot")]
#[derive(Clone, Debug)]
pub struct PyRecurrencePlot {
    inner: RecurrencePlot,
}

#[pymethods]
impl PyRecurrencePlot {
    fn __repr__(&self) -> String {
        format!(
            "RecurrencePlot(n_points={}, radius={:.4})",
            self.inner.n_points, self.inner.radius
        )
    }

    /// The binary recurrence matrix as a numpy.ndarray of dtype=bool.
    ///
    /// Shape is (n_points, n_points). True = recurrent.
    #[getter]
    fn matrix<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<bool>> {
        // Reconstruct as Vec<Vec<bool>> and use from_vec2 to avoid ndarray
        // version mismatches (workspace uses 0.15, numpy resolved against 0.16).
        let n = self.inner.n_points;
        let mat: Vec<Vec<bool>> = (0..n)
            .map(|i| (0..n).map(|j| self.inner.matrix[[i, j]]).collect())
            .collect();

        PyArray2::from_vec2(py, &mat).unwrap()
    }

    #[getter]
    fn radius(&self) -> f64 {
        self.inner.radius
    }

    #[getter]
    fn n_points(&self) -> usize {
        self.inner.n_points
    }

    /// Return a Python dict with the plot metadata and the recurrence matrix.
    ///
    /// Useful for serialization, pandas, or passing data around.
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("radius", self.inner.radius)?;
        dict.set_item("n_points", self.inner.n_points)?;
        dict.set_item("matrix", self.matrix(py))?;
        Ok(dict.into())
    }

    /// Construct a recurrence plot from a scalar time series (performs time-delay embedding internally).
    #[staticmethod]
    fn from_series(series: Vec<f64>, m: usize, tau: usize, radius: f64, theiler: usize) -> PyRecurrencePlot {
        let rp = RecurrencePlot::from_series(&series, m, tau, radius, theiler);
        PyRecurrencePlot { inner: rp }
    }

    /// Construct a recurrence plot from a pre-embedded trajectory.
    ///
    /// `trajectory` should be a list of lists (or result of `edim`).
    #[staticmethod]
    fn from_trajectory(trajectory: Vec<Vec<f64>>, radius: f64, theiler: usize) -> PyResult<PyRecurrencePlot> {
        if trajectory.is_empty() {
            return Ok(PyRecurrencePlot {
                inner: RecurrencePlot::new(),
            });
        }

        let n = trajectory.len();
        let dim = trajectory[0].len();
        let flat: Vec<f64> = trajectory.into_iter().flatten().collect();

        let arr = Array2::from_shape_vec((n, dim), flat)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid trajectory shape: {}", e)))?;

        let rp = RecurrencePlot::from_trajectory(&arr, radius, theiler);
        Ok(PyRecurrencePlot { inner: rp })
    }
}

impl From<RecurrencePlot> for PyRecurrencePlot {
    fn from(rp: RecurrencePlot) -> Self {
        Self { inner: rp }
    }
}

// --- Existing dynamics functions (unchanged) ---

#[pyfunction(name = "edim")]
pub fn py_edim(data: Vec<f64>, m: usize, tau: usize) -> Vec<Vec<f64>> {
    edim(&data, m, tau)
}

#[pyfunction(name = "fnn")]
pub fn py_fnn(data: Vec<f64>, m: usize, tau: usize, rtol: f64, atol: f64, theiler: usize) -> PyResult<(usize, f64)> {
    let result = fnn(&data, m, tau, rtol, atol, theiler);
    Ok((result.m, result.fnn_ratio))
}

#[pyfunction(name = "sample_entropy")]
pub fn py_sample_entropy(data: Vec<f64>, m: usize, r: f64) -> f64 {
    sample_entropy(&data, m, r)
}

// --- New RQA functions ---

#[pyfunction(name = "rqa")]
pub fn py_rqa(data: Vec<f64>, m: usize, tau: usize, radius: f64, theiler: usize) -> PyRqaResult {
    let res = rqa(&data, m, tau, radius, theiler);
    PyRqaResult::from(res)
}

#[pyfunction(name = "rqa_from_trajectory")]
pub fn py_rqa_from_trajectory(trajectory: Vec<Vec<f64>>, radius: f64, theiler: usize) -> PyResult<PyRqaResult> {
    if trajectory.is_empty() {
        return Ok(PyRqaResult::from(RqaResult::default()));
    }

    let n = trajectory.len();
    let dim = trajectory[0].len();
    let flat: Vec<f64> = trajectory.into_iter().flatten().collect();

    let arr = Array2::from_shape_vec((n, dim), flat)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid trajectory shape: {}", e)))?;

    let res = rqa_from_trajectory(&arr, radius, theiler);
    Ok(PyRqaResult::from(res))
}

#[pyfunction(name = "crqa")]
pub fn py_crqa(x: Vec<f64>, y: Vec<f64>, m: usize, tau: usize, radius: f64, theiler: usize) -> PyRqaResult {
    let res = crqa(&x, &y, m, tau, radius, theiler);
    PyRqaResult::from(res)
}

#[pyfunction(name = "multiscale_entropy")]
pub fn py_multiscale_entropy(data: Vec<f64>, max_scale: usize, m: usize, r: f64) -> Vec<f64> {
    multiscale_entropy(&data, max_scale, m, r)
}

/// Histogram `I(x; y)` in nats. Default `bins` is [`DEFAULT_AMI_BINS`].
#[pyfunction(name = "mutual_information")]
#[pyo3(signature = (x, y, bins = 8))]
pub fn py_mutual_information(x: Vec<f64>, y: Vec<f64>, bins: usize) -> f64 {
    mutual_information(&x, &y, bins)
}

/// Histogram mutual information on integer labels in `0..=255`.
#[pyfunction(name = "mutual_information_discrete")]
pub fn py_mutual_information_discrete(x: Vec<u32>, y: Vec<u32>) -> PyResult<f64> {
    let xb = labels_as_u8(&x)?;
    let yb = labels_as_u8(&y)?;
    Ok(mutual_information_discrete(&xb, &yb))
}

fn labels_as_u8(labels: &[u32]) -> PyResult<Vec<u8>> {
    labels
        .iter()
        .map(|&v| {
            u8::try_from(v).map_err(|_| pyo3::exceptions::PyValueError::new_err(format!("label {v} exceeds 255")))
        })
        .collect()
}

/// `I(x_t; x_{t+τ})` for `τ = 0..=max_lag`.
#[pyfunction(name = "ami")]
#[pyo3(signature = (x, max_lag, bins = 8))]
pub fn py_ami(x: Vec<f64>, max_lag: usize, bins: usize) -> Vec<f64> {
    ami(&x, max_lag, bins)
}

/// `I(driver[t−lag]; target[t])` for `lag = 0..=max_lag`.
#[pyfunction(name = "cross_ami")]
#[pyo3(signature = (driver, target, max_lag, bins = 8))]
pub fn py_cross_ami(driver: Vec<f64>, target: Vec<f64>, max_lag: usize, bins: usize) -> Vec<f64> {
    cross_ami(&driver, &target, max_lag, bins)
}

/// First interior lag where the curve has decreased and then levels or rises.
#[pyfunction(name = "first_local_minimum")]
pub fn py_first_local_minimum(values: Vec<f64>) -> Option<usize> {
    first_local_minimum(&values)
}

// ================================================
// Discrete transfer entropy
// ================================================

/// Histogram Schreiber TE config. Entropy in nats.
///
/// `k` / `l` are target / source history lengths. `tau` is the embedding
/// delay. `horizon` is the prediction step. `bins` is the number of
/// equal-occupancy quantile bins on the continuous path (ignored by
/// `transfer_entropy_discrete*`).
#[pyclass(name = "TeConfig")]
#[derive(Clone, Debug)]
pub struct PyTeConfig {
    #[pyo3(get, set)]
    pub k: usize,
    #[pyo3(get, set)]
    pub l: usize,
    #[pyo3(get, set)]
    pub tau: usize,
    #[pyo3(get, set)]
    pub horizon: usize,
    #[pyo3(get, set)]
    pub bins: usize,
}

#[pymethods]
impl PyTeConfig {
    #[new]
    #[pyo3(signature = (k = 1, l = 1, tau = 1, horizon = 1, bins = 4))]
    fn new(k: usize, l: usize, tau: usize, horizon: usize, bins: usize) -> Self {
        Self {
            k,
            l,
            tau,
            horizon,
            bins,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "TeConfig(k={}, l={}, tau={}, horizon={}, bins={})",
            self.k, self.l, self.tau, self.horizon, self.bins
        )
    }
}

impl From<&PyTeConfig> for TeConfig {
    fn from(c: &PyTeConfig) -> Self {
        Self {
            k: c.k,
            l: c.l,
            tau: c.tau,
            horizon: c.horizon,
            bins: c.bins,
        }
    }
}

fn default_te_cfg() -> TeConfig {
    TeConfig::default()
}

fn cfg_from(cfg: Option<&PyTeConfig>) -> TeConfig {
    cfg.map(TeConfig::from).unwrap_or_else(default_te_cfg)
}

/// Bivariate `TE(source → target)` with default `TeConfig`.
#[pyfunction(name = "transfer_entropy")]
pub fn py_transfer_entropy(source: Vec<f64>, target: Vec<f64>) -> f64 {
    transfer_entropy(&source, &target)
}

/// Bivariate TE with an explicit config.
#[pyfunction(name = "transfer_entropy_with")]
pub fn py_transfer_entropy_with(source: Vec<f64>, target: Vec<f64>, cfg: &PyTeConfig) -> f64 {
    transfer_entropy_with(&source, &target, &TeConfig::from(cfg))
}

/// Joint multi-source `TE((X1,...,Xp) → Y)`.
#[pyfunction(name = "transfer_entropy_mv")]
#[pyo3(signature = (sources, target, cfg=None))]
pub fn py_transfer_entropy_mv(sources: Vec<Vec<f64>>, target: Vec<f64>, cfg: Option<&PyTeConfig>) -> f64 {
    let refs: Vec<&[f64]> = sources.iter().map(|s| s.as_slice()).collect();
    transfer_entropy_mv(&refs, &target, &cfg_from(cfg))
}

/// Partial `TE((X1,...,Xp) → Y | Z1,...,Zq)`.
#[pyfunction(name = "transfer_entropy_conditional")]
#[pyo3(signature = (condition, sources, target, cfg=None))]
pub fn py_transfer_entropy_conditional(
    condition: Vec<Vec<f64>>,
    sources: Vec<Vec<f64>>,
    target: Vec<f64>,
    cfg: Option<&PyTeConfig>,
) -> f64 {
    let z: Vec<&[f64]> = condition.iter().map(|s| s.as_slice()).collect();
    let x: Vec<&[f64]> = sources.iter().map(|s| s.as_slice()).collect();
    transfer_entropy_conditional(&z, &x, &target, &cfg_from(cfg))
}

/// Bivariate TE on already-discrete labels (`0..=255`).
#[pyfunction(name = "transfer_entropy_discrete")]
#[pyo3(signature = (source, target, cfg=None))]
pub fn py_transfer_entropy_discrete(source: Vec<u32>, target: Vec<u32>, cfg: Option<&PyTeConfig>) -> PyResult<f64> {
    let xs = labels_as_u8(&source)?;
    let ys = labels_as_u8(&target)?;
    Ok(transfer_entropy_discrete(&xs, &ys, &cfg_from(cfg)))
}

/// Joint multi-source TE on already-discrete labels.
#[pyfunction(name = "transfer_entropy_discrete_mv")]
#[pyo3(signature = (sources, target, cfg=None))]
pub fn py_transfer_entropy_discrete_mv(
    sources: Vec<Vec<u32>>,
    target: Vec<u32>,
    cfg: Option<&PyTeConfig>,
) -> PyResult<f64> {
    let owned: Vec<Vec<u8>> = sources.iter().map(|s| labels_as_u8(s)).collect::<PyResult<_>>()?;
    let ys = labels_as_u8(&target)?;
    let refs: Vec<&[u8]> = owned.iter().map(|s| s.as_slice()).collect();
    Ok(transfer_entropy_discrete_mv(&refs, &ys, &cfg_from(cfg)))
}

/// Partial TE on already-discrete labels.
#[pyfunction(name = "transfer_entropy_discrete_conditional")]
#[pyo3(signature = (condition, sources, target, cfg=None))]
pub fn py_transfer_entropy_discrete_conditional(
    condition: Vec<Vec<u32>>,
    sources: Vec<Vec<u32>>,
    target: Vec<u32>,
    cfg: Option<&PyTeConfig>,
) -> PyResult<f64> {
    let z_owned: Vec<Vec<u8>> = condition.iter().map(|s| labels_as_u8(s)).collect::<PyResult<_>>()?;
    let x_owned: Vec<Vec<u8>> = sources.iter().map(|s| labels_as_u8(s)).collect::<PyResult<_>>()?;
    let ys = labels_as_u8(&target)?;
    let z: Vec<&[u8]> = z_owned.iter().map(|s| s.as_slice()).collect();
    let x: Vec<&[u8]> = x_owned.iter().map(|s| s.as_slice()).collect();
    Ok(transfer_entropy_discrete_conditional(&z, &x, &ys, &cfg_from(cfg)))
}

// ================================================
// Python register
// ================================================

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_edim, m)?)?;
    m.add_function(wrap_pyfunction!(py_fnn, m)?)?;
    m.add_function(wrap_pyfunction!(py_sample_entropy, m)?)?;

    // RQA + cRQA + MSE
    m.add_function(wrap_pyfunction!(py_rqa, m)?)?;
    m.add_function(wrap_pyfunction!(py_rqa_from_trajectory, m)?)?;
    m.add_function(wrap_pyfunction!(py_crqa, m)?)?;
    m.add_function(wrap_pyfunction!(py_multiscale_entropy, m)?)?;
    m.add_function(wrap_pyfunction!(py_mutual_information, m)?)?;
    m.add_function(wrap_pyfunction!(py_mutual_information_discrete, m)?)?;
    m.add_function(wrap_pyfunction!(py_ami, m)?)?;
    m.add_function(wrap_pyfunction!(py_cross_ami, m)?)?;
    m.add_function(wrap_pyfunction!(py_first_local_minimum, m)?)?;
    m.add("DEFAULT_AMI_BINS", DEFAULT_AMI_BINS)?;
    m.add_class::<PyRqaResult>()?;

    // RecurrencePlot (full object with matrix)
    m.add_class::<PyRecurrencePlot>()?;

    // Discrete transfer entropy
    m.add_class::<PyTeConfig>()?;
    m.add_function(wrap_pyfunction!(py_transfer_entropy, m)?)?;
    m.add_function(wrap_pyfunction!(py_transfer_entropy_with, m)?)?;
    m.add_function(wrap_pyfunction!(py_transfer_entropy_mv, m)?)?;
    m.add_function(wrap_pyfunction!(py_transfer_entropy_conditional, m)?)?;
    m.add_function(wrap_pyfunction!(py_transfer_entropy_discrete, m)?)?;
    m.add_function(wrap_pyfunction!(py_transfer_entropy_discrete_mv, m)?)?;
    m.add_function(wrap_pyfunction!(py_transfer_entropy_discrete_conditional, m)?)?;

    // Useful constants so Python users can match Rust defaults
    m.add("DEFAULT_LMIN", DEFAULT_LMIN)?;
    m.add("DEFAULT_VMIN", DEFAULT_VMIN)?;

    Ok(())
}
