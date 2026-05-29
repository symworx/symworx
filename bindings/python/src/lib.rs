// Copyright (c) 2026 SymWorx. All rights reserved.

use pyo3::prelude::*;

mod biosym;
mod core;
mod loadsym;

#[pymodule]
fn symworx(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let core_mod = PyModule::new(m.py(), "core")?;
    core::register(&core_mod)?;
    m.add_submodule(&core_mod)?;

    let biosym_mod = PyModule::new(m.py(), "biosym")?;
    biosym::register(&biosym_mod)?;
    m.add_submodule(&biosym_mod)?;

    let loadsym_mod = PyModule::new(m.py(), "loadsym")?;
    loadsym::register(&loadsym_mod)?;
    m.add_submodule(&loadsym_mod)?;

    Ok(())
}
