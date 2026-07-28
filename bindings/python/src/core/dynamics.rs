// Copyright (c) 2026 SymWorx

use ndarray::Array2;
use numpy::PyArray2;
use pyo3::{
    prelude::*,
    wrap_pyfunction,
};
use symworx_core::dynamics::{
    DEFAULT_LMIN,
    DEFAULT_VMIN,
    RecurrencePlot,
    RqaResult,
    crqa,
    edim,
    fnn,
    multiscale_entropy,
    rqa,
    rqa_from_trajectory,
    sample_entropy,
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
    m.add_class::<PyRqaResult>()?;

    // RecurrencePlot (full object with matrix)
    m.add_class::<PyRecurrencePlot>()?;

    // Useful constants so Python users can match Rust defaults
    m.add("DEFAULT_LMIN", DEFAULT_LMIN)?;
    m.add("DEFAULT_VMIN", DEFAULT_VMIN)?;

    Ok(())
}
