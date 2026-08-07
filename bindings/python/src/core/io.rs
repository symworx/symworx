// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

use pyo3::{
    exceptions::PyIOError,
    prelude::*,
    wrap_pyfunction,
};
use symworx_core::io::{
    csv::{
        CsvReader,
        CsvWriter,
    },
    gbd::GbdReader,
    load_any,
    read_ibi,
    traits::{
        SymReader,
        SymWriter,
    },
};

// ===========================================================
// load_any
// ===========================================================

#[pyfunction(name = "load_any")]
pub fn py_load_any(path: &str) -> PyResult<Vec<Vec<f64>>> {
    match load_any(path) {
        Ok(data) => Ok(data),
        Err(e) => Err(PyErr::new::<PyIOError, _>(format!("Failed to load file: {}", e))),
    }
}

// ===========================================================
// CSV Reader
// ===========================================================

#[pyclass(name = "CsvReader")]
pub struct PyCsvReader;

#[pymethods]
impl PyCsvReader {
    #[new]
    fn new() -> Self {
        PyCsvReader
    }

    pub fn read(&self, path: &str) -> PyResult<Vec<Vec<f64>>> {
        match CsvReader::read(path) {
            Ok(data) => Ok(data),
            Err(e) => Err(PyErr::new::<PyIOError, _>(format!("Failed to read CSV file: {}", e))),
        }
    }
}

// ===========================================================
// CSV Writer
// ===========================================================
#[pyclass(name = "CsvWriter")]
pub struct PyCsvWriter;

#[pymethods]
impl PyCsvWriter {
    #[new]
    fn new() -> Self {
        PyCsvWriter
    }

    pub fn write(&self, path: &str, data: Vec<Vec<f64>>) -> PyResult<()> {
        match CsvWriter::write(path, &data) {
            Ok(_) => Ok(()),
            Err(e) => Err(PyErr::new::<PyIOError, _>(format!("Failed to write CSV file: {}", e))),
        }
    }
}

// ===========================================================
// GBD (stub)
// ===========================================================

#[pyclass(name = "GbdTable")]
pub struct PyGbdTable {
    #[pyo3(get)]
    pub name: String,

    #[pyo3(get)]
    pub rows: Vec<Vec<String>>,
}

#[pyfunction(name = "read_gbd")]
pub fn py_read_gbd(path: &str, sql: &str) -> PyResult<PyGbdTable> {
    match GbdReader::query(path, sql) {
        Ok(table) => Ok(PyGbdTable {
            name: table.name,
            rows: table.rows,
        }),
        Err(e) => Err(PyErr::new::<PyIOError, _>(format!("Failed to read GBD: {}", e))),
    }
}

// ===========================================================
// IBI
// ===========================================================

#[pyclass(name = "IbiRecord")]
pub struct PyIbiRecord {
    #[pyo3(get)]
    pub timestamp: u32,

    #[pyo3(get)]
    pub rr_ms: u16,
}

#[pyfunction(name = "read_ibi")]
pub fn py_read_ibi(path: &str) -> PyResult<Vec<PyIbiRecord>> {
    match read_ibi(path) {
        Ok(records) => Ok(records
            .into_iter()
            .map(|r| PyIbiRecord {
                timestamp: r.timestamp,
                rr_ms: r.rr_ms,
            })
            .collect()),
        Err(e) => Err(PyErr::new::<PyIOError, _>(format!("Failed to read IBI file: {}", e))),
    }
}

// ===========================================================
// Parquet (stub)
// ===========================================================

#[pyclass(name = "ParquetReader")]
pub struct PyParquetReader;

// ===========================================================
// Activity / FIT (for LoadSym rides: SRM/Garmin/Polar)
// Lightweight exposure so external tools / hybrid projects can call
// symworx for parsing without duplicating parsers.
// ===========================================================

#[pyclass(name = "ActivityData")]
pub struct PyActivityData {
    #[pyo3(get)]
    pub source: String,
    #[pyo3(get)]
    pub manufacturer: Option<String>,
    #[pyo3(get)]
    pub sport: Option<String>,
    #[pyo3(get)]
    pub n_samples: usize,
    #[pyo3(get)]
    pub duration_s: f64,
    #[pyo3(get)]
    pub has_power: bool,
    // Note: full series available via methods or as dict for simplicity
}

#[pymethods]
impl PyActivityData {
    fn __repr__(&self) -> String {
        format!(
            "ActivityData(source={}, samples={}, duration={:.1}s, has_power={})",
            self.source, self.n_samples, self.duration_s, self.has_power
        )
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        // For full data, re-load or extend later. This is lightweight.
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("source", &self.source)?;
        dict.set_item("manufacturer", &self.manufacturer)?;
        dict.set_item("sport", &self.sport)?;
        dict.set_item("n_samples", self.n_samples)?;
        dict.set_item("duration_s", self.duration_s)?;
        dict.set_item("has_power", self.has_power)?;
        Ok(dict.into())
    }
}

#[pyfunction(name = "load_activity")]
pub fn py_load_activity(path: &str) -> PyResult<PyObject> {
    use pyo3::{
        Python,
        types::PyDict,
    };
    // symworx-io is declared as symworx-io in Cargo → identifier symworx_io
    match symworx_io::load_activity(path) {
        Ok(act) => Python::with_gil(|py| {
            // Return a simple dict for backward compat + rich info
            let d = PyDict::new(py);
            let _ = d.set_item("source", &act.source);
            let _ = d.set_item("manufacturer", &act.manufacturer);
            let _ = d.set_item("sport", &act.sport);
            let _ = d.set_item("n_samples", act.times_s.len());
            let _ = d.set_item("duration_s", act.duration_s());
            let p: Vec<Option<f64>> = act.power_w.clone();
            let _ = d.set_item("power_w", p);
            let _ = d.set_item("has_power", act.has_power());
            Ok(d.into())
        }),
        Err(e) => Err(PyErr::new::<PyIOError, _>(format!("load_activity: {}", e))),
    }
}

// ===========================================================
// Email / IMAP fetching (optional; needs `email` feature + OpenSSL)
// ===========================================================

#[cfg(feature = "email")]
#[pyfunction(name = "fetch_srm_fit_attachments")]
pub fn py_fetch_srm_fit_attachments(user: &str, app_password: &str, target_dir: &str) -> PyResult<Vec<String>> {
    match symworx_io::fetch_srm_fit_attachments(user, app_password, std::path::Path::new(target_dir)) {
        Ok(paths) => Ok(paths.into_iter().map(|p| p.to_string_lossy().to_string()).collect()),
        Err(e) => Err(PyErr::new::<PyIOError, _>(format!("fetch_srm_fit_attachments: {}", e))),
    }
}

// ===========================================================
// PYTHON REGISTER
// ===========================================================

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_read_gbd, m)?)?;
    m.add_function(wrap_pyfunction!(py_read_ibi, m)?)?;
    m.add_function(wrap_pyfunction!(py_load_activity, m)?)?;
    #[cfg(feature = "email")]
    m.add_function(wrap_pyfunction!(py_fetch_srm_fit_attachments, m)?)?;

    m.add_class::<PyCsvReader>()?;
    m.add_class::<PyCsvWriter>()?;
    m.add_class::<PyGbdTable>()?;
    m.add_class::<PyIbiRecord>()?;
    m.add_class::<PyParquetReader>()?;
    m.add_class::<PyActivityData>()?;

    Ok(())
}
