// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Physiology module.
//!
//!

pub mod ppg;
pub mod respiration;

// EXPORTS
pub use ppg::{
    PPGNoiseConfig, PPGSignalQuality, PPGSimulationParams, PPGTimeSeries, analyze_ppg,
    generate_ppg_timeseries, generate_ppg_waveform,
};
pub use respiration::{RespSimulationParams, RespTimeSeries, generate_respiration_timeseries};
