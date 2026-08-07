// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! # symworx-biosym
//!
//! Biological signals, biomechanics, and coupled physiological modeling crate
//! within the SymWorx ecosystem.
//!
//! ## Core Modules
//!
//! - [`biomechanics`] — Gait analysis (`biomechanics::gait`), spatiotemporal metrics,
//!   stride/step length, symmetry, and related data models.
//! - [`biosystems`] — Cross-domain and integrative modeling frameworks
//!   (coupled oscillators / CPG, fatigue, intensity, run performance models, etc.).
//! - [`physiology`] — Foundational physiological signal processing (PPG,
//!   respiration, etc.).
//!
//! ## Features
//!
//! - Idiomatic Rust core with minimal dependencies
//! - Fixed-step RK4 integration (sourced from `symworx-math`)
//! - Also available under the unified `symworx.biosym` namespace
//!
//! ## Quick Start (Rust)
//!
//! ```rust
//! use symworx_biosym::{GaitParams, SymCpgModel};
//!
//! let params = GaitParams::default().with_defaults();
//! let model = SymCpgModel::new(None); // default config
//! let (times, states) = model.run((0.0, 60.0), 0.01);
//! ```

// #![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/symworx-biosym")]

// Modules

/// Gait analysis and modeling primitives.
///
/// Provides `GaitParams`, `GaitData`, `GaitStats`, `GaitAnalysis`, stride
/// detection, symmetry, and related spatiotemporal metrics. Also available
/// under the `biomechanics` grouping.
pub mod gait;

/// Cross-domain and integrative modeling frameworks.
///
/// Home for blended models that cross physiology and biomechanics boundaries:
/// coupled oscillators (CPG), fatigue, intensity, run performance, etc.
/// The previous `cpg` module lives here (see the `cpg` compatibility shim below).
pub mod biosystems;

/// PPG generation and analysis.
pub mod ppg;

/// Respiration generation and analysis.
pub mod respiration;

/// Grouped access to physiological signal tools (PPG + respiration).
///
/// Provided for discoverability and backward compatibility. All items are
/// also available directly via the flat modules (`ppg`, `respiration`) or
/// at the crate root.
#[allow(ambiguous_glob_reexports)]
pub mod physiology {
    // Re-exports so `physiology::PPGTimeSeries`, `physiology::analyze_ppg`, etc. continue to work.
    // Glob re-exports intentionally overlap (common helpers via both ppg and respiration).
    pub use crate::{
        common,
        ppg,
        ppg::*,
        respiration,
        respiration::*,
    };
}

/// Grouped access to biomechanical modeling tools.
///
/// Currently contains `gait`. The RunSym functionality (runner/shoe modeling,
/// running performance simulation, fatigue/intensity effects on locomotion, etc.)
/// will be developed as additional modules under the biomechanics domain.
///
/// All items are also available directly via the flat modules (e.g. `gait`) or at the
/// crate root.
#[allow(ambiguous_glob_reexports)]
pub mod biomechanics {
    pub use crate::gait;
    // Re-exports so `biomechanics::GaitParams`, `biomechanics::analyze_gait`, etc. continue to work.
    pub use crate::gait::*;
}

/// Shared cross-domain primitives.
///
/// `IntervalSeries`, processing parameters, signal containers, peak helpers,
/// HRV, etc. Preferred path for new code. Used by physiology, biomechanics,
/// and biosystems so that domains do not depend directly on each other.
pub mod common;

// Re-exports at the crate root for a convenient "batteries-included" experience.
// New code is encouraged to use the flat modules (`gait`, `ppg`, `respiration`, `biosystems`)
// or the named groupings (`biomechanics`, `physiology`) below.
// Overlapping globs are intentional for discoverability; prefer module paths in new code.
#[allow(ambiguous_glob_reexports)]
pub use biosystems::*;
pub use common::{
    IntervalSeries,
    processing::{
        BandpassParams,
        PeakDetectionParams,
        PhysiologyProcessingParams,
        apply_bandpass,
        apply_peak_overrides,
    },
};
#[allow(ambiguous_glob_reexports)]
pub use gait::*;
#[allow(ambiguous_glob_reexports)]
pub use ppg::*;
#[allow(ambiguous_glob_reexports)]
pub use respiration::*;

// Temporary compatibility shim for code that still uses the old `cpg` path.
// Preferred: `symworx_biosym::biosystems` (or `biosystems::cpg`).
pub mod cpg {
    pub use crate::biosystems::cpg::*;
}

// Temporary compatibility shim so `crate::processing::...` references continue to work.
// Preferred: `crate::common::processing`.
pub mod processing {
    pub use crate::common::processing::*;
}

// Version info
/// Current version of the `symworx-biosym` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
