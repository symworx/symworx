// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Cross-domain and integrative biological systems modeling.
//!
//! This is the home for blended modeling frameworks that cross traditional
//! physiology / biomechanics boundaries (coupled oscillators, fatigue and
//! intensity modulators, run performance simulation, etc.).
//!
//! ## Submodules
//!
//! - [`cpg`] — Coupled Van der Pol oscillators (heart + bilateral legs + respiration)
//!   with dynamic effort (`tau`) modulation.
//! - Fatigue, intensity, and run performance models (work in progress; ported
//!   from legacy runsym intent).
//!
//! Items are also re-exported at the crate root for convenience
//! (`symworx_biosym::SymCpgModel`, etc.).

pub mod cpg;
pub mod fatigue;
pub mod intensity;
pub mod run_performance;

// Re-export the primary CPG types at the biosystems level for ergonomics
// (matching previous top-level exposure).
pub use cpg::{
    CpgConfig,
    SymCpgModel,
    instantaneous_freq,
};
