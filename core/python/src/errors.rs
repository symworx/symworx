use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use core::errors::SymError;

pub fn symerror_to_py(err: SymError) -> PyErr {
    PyValueError::new_err(err.to_string())
}
