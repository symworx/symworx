// biosym/src/physiology/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

// ==========================================================
// MODULES
// ==========================================================
pub mod ppg;
// pub mod respiration;

// ==========================================================
// EXPORTS
// ==========================================================
pub use ppg::{
    analyze_ppg,
    PPGTimeSeries,
    create_ppg_waveform,
    create_ppg_timeseries,
    PPGNoiseConfig,
    PPGSignalQuality,
};
// pub use respiration::{
//     analyze_respiration,
//     RespTimeSeries,
//     create_respiration_waveform,
//     create_respiration_timeseries,
//     RespNoiseConfig,
//     RespSignalQuality,
// };

// ==========================================================
// PYTHON REGISTER
// ==========================================================
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {

    // --- PPG ----------------------------------------------
    // m.add_function(wrap_pyfunction!(py_analyze_ppg, m)?)?;
    // m.add_function(wrap_pyfunction!(py_create_ppg_waveform, m)?)?;
    // m.add_function(wrap_pyfunction!(py_create_ppg_timeseries, m)?)?;
    // m.add_class::<PyPPGTimeSeries>()?;
    // m.add_class::<PyPPGNoiseConfig>()?;
    // m.add_class::<PyPPGSignalQuality>()?;

    // --- Respiration --------------------------------------
    // m.add_function(wrap_pyfunction!(py_analyze_respiration, m)?)?;
    // m.add_function(wrap_pyfunction!(py_create_respiration_waveform, m)?)?;
    // m.add_function(wrap_pyfunction!(py_create_respiration_timeseries, m)?)?;
    // m.add_class::<PyRespTimeSeries>()?;
    // m.add_class::<PyRespNoiseConfig>()?;
    // m.add_class::<PyRespSignalQuality>()?;

    Ok(())
}
