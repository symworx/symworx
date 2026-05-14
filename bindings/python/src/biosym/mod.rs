// symworx/bindings/python/src/biosym/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;

pub mod physiology;
// pub mod biomechanics;
// pub mod optimization;

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {

    // let biomechanics_mod = PyModule::new(m.py(), "biomechanics")?;
    // biomechanics::register(&biomechanics_mod)?;
    // m.add_submodule(&biomechanics_mod)?;

    // let cpg_mod = PyModule::new(m.py(), "cpg")?;
    // cpg::register(&cpg_mod)?;
    // m.add_submodule(&cpg_mod)?;

    let physiology_mod = PyModule::new(m.py(), "physiology")?;
    physiology::register(&physiology_mod)?;
    m.add_submodule(&physiology_mod)?;

    Ok(())
}
