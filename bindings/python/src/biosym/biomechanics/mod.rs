// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

use pyo3::prelude::*;

pub mod gait;

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    gait::register(m)?;

    Ok(())
}
