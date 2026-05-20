// symworx/bindings/python/src/biosym/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;

pub mod physiology;

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Re-export everything from the standalone symworx_biosym package
    // so users can do: import symworx.biosym as biosym
    let biosym_mod = m.py().import("symworx_biosym")?;
    m.add_submodule(&biosym_mod)?;

    // Also keep physiology submodule if it has extra content
    let physiology_mod = PyModule::new(m.py(), "physiology")?;
    physiology::register(&physiology_mod)?;
    m.add_submodule(&physiology_mod)?;

    Ok(())
}
