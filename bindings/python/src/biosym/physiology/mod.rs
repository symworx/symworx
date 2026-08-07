// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

use pyo3::prelude::*;

pub mod ppg;
pub mod respiration;

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    ppg::register(m)?;
    respiration::register(m)?;
    Ok(())
}
