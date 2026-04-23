use pyo3::prelude::*;
use core::io::load_any;
use crate::errors::symerror_to_py;

#[pyfunction]
pub fn py_load_any(path: &str) -> PyResult<Vec<Vec<f64>>> {
    load_any(path).map_err(symerror_to_py)
}
