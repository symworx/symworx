// Copyright (c) 2026 SymWorx

use pyo3::prelude::*;

pub mod biomechanics;
pub mod cpg;
pub mod physiology;

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Biomechanics submodule
    let biomechanics_mod = PyModule::new(m.py(), "biomechanics")?;
    biomechanics::register(&biomechanics_mod)?;
    m.add_submodule(&biomechanics_mod)?;

    // CPG submodule
    let cpg_mod = PyModule::new(m.py(), "cpg")?;
    cpg::register(&cpg_mod)?;
    m.add_submodule(&cpg_mod)?;

    // Physiology submodule
    let physiology_mod = PyModule::new(m.py(), "physiology")?;
    physiology::register(&physiology_mod)?;
    m.add_submodule(&physiology_mod)?;

    Ok(())
}
