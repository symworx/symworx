// biosym/src/lib.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(improper_ctypes_definitions)]

use pyo3::prelude::*;

mod physiology;

// ========================================================
// PYTHON MODULE
// ========================================================
#[pymodule]
fn biosym(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // --- Statistics submodule ---------------------------------
    let physiology_mod = PyModule::new_bound(py, "physiology")?;
    crate::physiology::register(py, &physiology_mod)?;
    m.add_submodule(&physiology_mod)?;

    Ok(())
}
