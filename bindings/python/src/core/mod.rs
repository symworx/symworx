// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

use pyo3::prelude::*;

pub mod dynamics;
pub mod filters;
pub mod io;
pub mod math;
pub mod processing;
pub mod statistics;
// pub mod backend;

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let dynamics_mod = PyModule::new(m.py(), "dynamics")?;
    dynamics::register(&dynamics_mod)?;
    m.add_submodule(&dynamics_mod)?;

    let filters_mod = PyModule::new(m.py(), "filters")?;
    filters::register(&filters_mod)?;
    m.add_submodule(&filters_mod)?;

    let io_mod = PyModule::new(m.py(), "io")?;
    io::register(&io_mod)?;
    m.add_submodule(&io_mod)?;

    let processing_mod = PyModule::new(m.py(), "processing")?;
    processing::register(&processing_mod)?;
    m.add_submodule(&processing_mod)?;

    let statistics_mod = PyModule::new(m.py(), "statistics")?;
    statistics::register(&statistics_mod)?;
    m.add_submodule(&statistics_mod)?;

    let math_mod = PyModule::new(m.py(), "math")?;
    math::register(&math_mod)?;
    m.add_submodule(&math_mod)?;

    Ok(())
}
