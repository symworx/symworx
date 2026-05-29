// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

pub mod analysis;
pub mod generation;
pub mod noise;
pub mod quality;

pub use analysis::analyze_ppg;
pub use generation::{
    PPGSimulationParams, PPGTimeSeries, generate_ppg_timeseries, generate_ppg_waveform,
};
pub use noise::PPGNoiseConfig;
pub use quality::PPGSignalQuality;
