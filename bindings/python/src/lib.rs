// Copyright (c) 2026 SymWorx

use pyo3::prelude::*;

mod biosym;
mod core;
mod loadsym;

/// Attach `child` on `parent` and register one public import path in `sys.modules`.
///
/// - Attribute access: `_lib.biosym` (via `add_submodule`)
/// - Import path: `import symworx.biosym` (via `sys.modules["symworx.biosym"]`)
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

/// Compiled extension entry point (`module-name = "symworx._lib"` in pyproject.toml).
///
/// Pure-Python package code under `symworx/` re-exports from here so the package
/// directory does not shadow the native module.
#[pymodule]
fn _lib(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let core_mod = PyModule::new(m.py(), "core")?;
    core::register(&core_mod)?;
    attach_submodule(m, &core_mod, "symworx.core")?;

    let biosym_mod = PyModule::new(m.py(), "biosym")?;
    biosym::register(&biosym_mod)?;
    attach_submodule(m, &biosym_mod, "symworx.biosym")?;

    let loadsym_mod = PyModule::new(m.py(), "loadsym")?;
    loadsym::register(&loadsym_mod)?;
    attach_submodule(m, &loadsym_mod, "symworx.loadsym")?;

    Ok(())
}
