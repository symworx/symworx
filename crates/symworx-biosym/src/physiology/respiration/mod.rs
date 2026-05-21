// symworx/crates/symworx-biosym/src/physiology/respiration/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

pub mod generation;

pub use generation::{
    RespSimulationParams,
    RespTimeSeries,
    generate_respiration_timeseries,
};
