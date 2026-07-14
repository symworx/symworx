// Copyright (c) 2026 SymWorx

use pyo3::prelude::*;

pub mod load;
pub mod nutrition;

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
    let load_mod = PyModule::new(m.py(), "load")?;
    load::register(&load_mod)?;
    attach_submodule(m, &load_mod, "symworx.loadsym.load")?;

    let nutrition_mod = PyModule::new(m.py(), "nutrition")?;
    nutrition::register(&nutrition_mod)?;
    attach_submodule(m, &nutrition_mod, "symworx.loadsym.nutrition")?;

    Ok(())
}
