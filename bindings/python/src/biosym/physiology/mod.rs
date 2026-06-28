// Copyright (c) 2026 SymWorx

use pyo3::prelude::*;

pub mod ppg;
pub mod respiration;

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    ppg::register(m)?;
    respiration::register(m)?;
    Ok(())
}
