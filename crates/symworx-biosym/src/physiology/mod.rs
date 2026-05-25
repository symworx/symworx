// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Physiology module.
//!
//! 

pub mod ppg;
pub mod respiration;

// EXPORTS
pub use ppg::{
    analyze_ppg,
    PPGSimulationParams,
    PPGTimeSeries,
    generate_ppg_waveform,
    generate_ppg_timeseries,
    PPGNoiseConfig,
    PPGSignalQuality,
};
pub use respiration::{
    RespSimulationParams,
    RespTimeSeries,
    generate_respiration_timeseries,
};
