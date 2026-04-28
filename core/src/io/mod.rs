// core/src/io/mod.rs
// Copyright (C) 2026 cSYMd

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

// ===========================================================
// MODULES
// ===========================================================
pub mod csv;
pub mod gbd;
pub mod ibi;
pub mod parquet;
pub mod traits;
pub mod py;

// ===========================================================
// EXPORTS 
// ===========================================================
pub use csv::{CsvReader, CsvWriter};
pub use gbd::{GbdReader, GbdTable};
pub use ibi::{IbiRecord, read_ibi};
pub use parquet::ParquetReader;

use crate::errors::SymError;
use crate::io::traits::{SymReader, SymWriter};

// ===========================================================
// Parent module functions
// ===========================================================
pub fn load_any(path: &str) -> Result<Vec<Vec<f64>>, SymError> {
    if path.ends_with(".csv") {
        CsvReader::read(path)
    } else if path.ends_with(".parquet") {
        ParquetReader::read(path)
    } else {
        Err(SymError::UnsupportedFormat(path.into()))
    }
}

// ===========================================================
// PYTHON EXPORTS
// ===========================================================
pub use py::{
    py_load_any,
    PyCsvReader,
    PyCsvWriter,
    PyGbdTable,
    py_read_gbd,
    PyIbiRecord,
    py_read_ibi,
    PyParquetReader,
};

// ===========================================================
// PYTHON REGISTER
// ===========================================================
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {

    // --- load_any -----------------------------------------
    m.add_function(pyo3::wrap_pyfunction!(py_load_any, m)?)?;

    // --- CSV ----------------------------------------------
    m.add_class::<PyCsvReader>()?;
    m.add_class::<PyCsvWriter>()?;

    // --- GBD ----------------------------------------------
    m.add_class::<PyGbdTable>()?;
    m.add_function(wrap_pyfunction!(py_read_gbd, m)?)?;

    // --- IBI ----------------------------------------------
    m.add_class::<PyIbiRecord>()?;
    m.add_function(wrap_pyfunction!(py_read_ibi, m)?)?;

    // --- Parquet ------------------------------------------
    m.add_class::<PyParquetReader>()?;

    Ok(())
}
