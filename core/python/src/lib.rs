// core/python/src/lib.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;

mod io;
mod errors;

mod filters;
mod statistics;
mod processing;

#[pymodule]
fn csymd_core(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // --- Backend submodule ------------------------------------
    let backend_mod = PyModule::new_bound(py, "backend")?;
    m.add_submodule(&backend_mod)?;

    // --- Filters submodule ------------------------------------
    let filters_mod = PyModule::new_bound(py, "filters")?;
    // Only add functions if they exist:
    // filters_mod.add_function(wrap_pyfunction!(filters::py_moving_average, &filters_mod)?)?;
    m.add_submodule(&filters_mod)?;

    // --- IO submodule -----------------------------------------
    let io_mod = PyModule::new_bound(py, "io")?;
    io_mod.add_function(wrap_pyfunction!(io::py_load_any, &io_mod)?)?;
    m.add_submodule(&io_mod)?;

    // --- Processing submodule ---------------------------------
    let processing_mod = PyModule::new_bound(py, "processing")?;
    m.add_submodule(&processing_mod)?;

    // --- Statistics submodule ---------------------------------
    let stats_mod = PyModule::new_bound(py, "statistics")?;
    m.add_submodule(&stats_mod)?;

    // --- Errors submodule -------------------------------------
    //
    // DO NOT expose symerror_to_py as a Python function.
    // It is an internal converter, not a Python API.
    //
    let errors_mod = PyModule::new_bound(py, "errors")?;
    m.add_submodule(&errors_mod)?;

    Ok(())
}
