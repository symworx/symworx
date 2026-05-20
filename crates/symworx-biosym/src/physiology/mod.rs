// biosym/src/physiology/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

pub mod ppg;
pub mod respiration;

// ==========================================================
// EXPORTS
// ==========================================================
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
