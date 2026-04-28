// core/src/io/py.rs
// Copyright (C) 2026 cSYMd

use pyo3::prelude::*;
use pyo3::exceptions::PyIOError;

use crate::io::traits::{SymReader, SymWriter};

// ===========================================================
// load_any
// ===========================================================
#[pyfunction(name = "load_any")]
pub fn py_load_any(path: &str) -> PyResult<Vec<Vec<f64>>> {
    match crate::io::load_any(path) {
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
        match crate::io::CsvReader::read(path) {
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
        match crate::io::csv::CsvWriter::write(path, &data) {
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
    match crate::io::gbd::GbdReader::query(path, sql) {
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
    match crate::io::read_ibi(path) {
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
