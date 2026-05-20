// biosym/src/physiology/ppg/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

pub mod analysis;
pub mod generation;
pub mod noise;
pub mod quality;

pub use analysis::analyze_ppg;
pub use generation::{
    PPGSimulationParams,
    PPGTimeSeries,
    generate_ppg_waveform,
    generate_ppg_timeseries,
};
pub use noise::PPGNoiseConfig;
pub use quality::PPGSignalQuality;
