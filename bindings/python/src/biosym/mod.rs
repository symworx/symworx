// Copyright (c) 2026 SymWorx

use pyo3::prelude::*;

pub mod biomechanics;
pub mod cpg;
pub mod physiology;

/// Attach child on parent + one public `sys.modules` entry (same pattern as `lib.rs`).
fn attach_submodule(
    parent: &Bound<'_, PyModule>,
    child: &Bound<'_, PyModule>,
    public_import_path: &str,
) -> PyResult<()> {
    parent.add_submodule(child)?;
    let py = parent.py();
    py.import("sys")?
        .getattr("modules")?
        .set_item(public_import_path, child)?;
    Ok(())
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let biomechanics_mod = PyModule::new(m.py(), "biomechanics")?;
    biomechanics::register(&biomechanics_mod)?;
    attach_submodule(m, &biomechanics_mod, "symworx.biosym.biomechanics")?;
    // Ergonomic top-level: `biosym.GaitParams` as well as `biosym.biomechanics.GaitParams`.
    biomechanics::gait::register(m)?;

    let cpg_mod = PyModule::new(m.py(), "cpg")?;
    cpg::register(&cpg_mod)?;
    attach_submodule(m, &cpg_mod, "symworx.biosym.cpg")?;

    let physiology_mod = PyModule::new(m.py(), "physiology")?;
    physiology::register(&physiology_mod)?;
    attach_submodule(m, &physiology_mod, "symworx.biosym.physiology")?;

    Ok(())
}
