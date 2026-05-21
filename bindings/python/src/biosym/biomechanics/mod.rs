// symworx/bindings/python/src/biosym/biomechanics/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;

pub mod gait;

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    gait::register(m)?;

    Ok(())
}
