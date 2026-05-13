// symworx/bindings/python/src/lib.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;

mod core;
mod biosym;
mod loadsym;
// mod runsym;

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
