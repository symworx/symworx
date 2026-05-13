// biosym/src/physiology/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

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
