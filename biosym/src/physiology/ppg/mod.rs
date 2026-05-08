// biosym/src/physiology/ppg/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

//! The PPG module (symworx_biosym.physiology) 
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

// ==========================================================
// MODULES
// ==========================================================
pub mod analysis;
pub mod generation;
pub mod noise;
pub mod quality;

// ==========================================================
// EXPORTS
// ==========================================================
pub use analysis::analyze_ppg;
pub use generation::{PPGTimeSeries, create_ppg_waveform, create_ppg_timeseries,};
pub use noise::PPGNoiseConfig;
pub use quality::PPGSignalQuality;
