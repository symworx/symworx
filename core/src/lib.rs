// core/src/lib.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(improper_ctypes_definitions)]

use pyo3::prelude::*;

mod io;
mod math;
mod errors;
mod filters;
mod dynamics;
mod statistics;
mod processing;

// ========================================================
// PYTHON MODULE
// ========================================================
#[pymodule]
fn core(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // --- Backend submodule ---------------------------------
    // let backend_mod = PyModule::new_bound(py, "backend")?;
    // backend::register(py, &backend_mod)?;
    // m.add_submodule(&backend_mod)?;

    // --- Dynamics submodule ---------------------------------
    let dynamics_mod = PyModule::new_bound(py, "dynamics")?;
    crate::dynamics::register(py, &dynamics_mod)?;
    m.add_submodule(&dynamics_mod)?;

    // --- Filters submodule ---------------------------------
    let filters_mod = PyModule::new_bound(py, "filters")?;
    crate::filters::register(py, &filters_mod)?;
    m.add_submodule(&filters_mod)?;

    // --- IO submodule -----------------------------------------
    let io_mod = PyModule::new_bound(py, "io")?;
    crate::io::register(py, &io_mod)?;
    m.add_submodule(&io_mod)?;

    // --- Math submodule -----------------------------------------
    let math_mod = PyModule::new_bound(py, "math")?;
    crate::math::register(py, &math_mod)?;
    m.add_submodule(&math_mod)?;
    
    // --- Processing submodule ---------------------------------
    let processing_mod = PyModule::new_bound(py, "processing")?;
    crate::processing::register(py, &processing_mod)?;
    m.add_submodule(&processing_mod)?;

    // --- Statistics submodule ---------------------------------
    let stats_mod = PyModule::new_bound(py, "statistics")?;
    crate::statistics::register(py, &stats_mod)?;
    m.add_submodule(&stats_mod)?;

    // --- Errors submodule -------------------------------------
    // let errors_mod = PyModule::new_bound(py, "errors")?;
    // errors::register(py, &errors_mod)?;
    // m.add_submodule(&errors_mod)?;

    Ok(())
}
