// core/src/lib.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(improper_ctypes_definitions)]

use pyo3::prelude::*;

mod io;
mod errors;
mod filters;
mod dynamics;
mod statistics;
mod processing;

//
// === PYTHON MODULE ======================================
// 
#[pymodule]
fn csymd_core(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // --- Backend submodule ---------------------------------
    // let backend_mod = PyModule::new_bound(py, "backend")?;
    // backend::register(py, &backend_mod)?;
    // m.add_submodule(&backend_mod)?;

    // --- Dynamics submodule ---------------------------------
    let dynamics_mode = PyModule::new_bound(py, "dynamics")?;
    dynamics::register(py, &dynamics_mode)?;
    m.add_submodule(&dynamics_mode)?;

    // --- Filters submodule ---------------------------------
    // let filters_mod = PyModule::new_bound(py, "filters")?;
    // filters::register(py, &filters_mod)?;
    // m.add_submodule(&filters_mod)?;

    // --- IO submodule -----------------------------------------
    // let io_mod = PyModule::new_bound(py, "io")?;
    // io::register(py, &io_mod)?;
    // m.add_submodule(&io_mod)?;

    // --- Processing submodule ---------------------------------
    // let processing_mod = PyModule::new_bound(py, "processing")?;
    // processing::register(py, &processing_mod)?;
    // m.add_submodule(&processing_mod)?;

    // --- Statistics submodule ---------------------------------
    let stats_mod = PyModule::new_bound(py, "statistics")?;
    statistics::register(py, &stats_mod)?;
    m.add_submodule(&stats_mod)?;

    // --- Errors submodule -------------------------------------
    // let errors_mod = PyModule::new_bound(py, "errors")?;
    // errors::register(py, &errors_mod)?;
    // m.add_submodule(&errors_mod)?;

    Ok(())
}
