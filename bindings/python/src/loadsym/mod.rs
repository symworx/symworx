// Copyright (c) 2026 SymWorx

use pyo3::prelude::*;

pub mod load;
pub mod nutrition;

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let load_mod = PyModule::new(m.py(), "load")?;
    load::register(&load_mod)?;
    m.add_submodule(&load_mod)?;

    let nutrition_mod = PyModule::new(m.py(), "nutrition")?;
    nutrition::register(&nutrition_mod)?;
    m.add_submodule(&nutrition_mod)?;

    Ok(())
}
