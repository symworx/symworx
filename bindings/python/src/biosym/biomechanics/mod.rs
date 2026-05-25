// Copyright (c) 2026 SymWorx. All rights reserved.

use pyo3::prelude::*;

pub mod gait;

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    gait::register(m)?;

    Ok(())
}
