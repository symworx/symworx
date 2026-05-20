// symworx/bindings/python/src/biosym/physiology/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;

pub mod ppg;
pub mod respiration;

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    ppg::register(m)?;
    respiration::register(m)?;
    Ok(())
}