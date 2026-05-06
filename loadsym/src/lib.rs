// loadsym/src/lib.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

#![allow(unused_imports)]
#![allow(unsafe_op_in_unsafe_fn)]

use pyo3::prelude::*;

mod load;
mod nutrition;

// ========================================================
// PYTHON MODULE
// ========================================================
#[pymodule]
fn csymd_loadsym(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {

    // --- Load submodule ---------------------------------
    let load_mod = PyModule::new_bound(py, "load")?;
    load::register(py, &load_mod)?;
    m.add_submodule(&load_mod)?;

    // --- Nutrition submodule ---------------------------------
    let nutrition_mod = PyModule::new_bound(py, "nutrition")?;
    nutrition::register(py, &nutrition_mod)?;
    m.add_submodule(&nutrition_mod)?;

    Ok(())
}
