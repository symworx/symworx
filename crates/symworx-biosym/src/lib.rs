// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! # symworx-biosym
//!
//! Biological signals, biomechanics, and coupled physiological modeling crate
//! within the SymWorx ecosystem.
//!
//! ## Core Modules
//!
//! - [`biomechanics`] — Gait analysis (`biomechanics::gait`), spatiotemporal metrics,
//!   stride/step length, symmetry, and related data models.
//! - [`cpg`] — Central Pattern Generator using coupled Van der Pol oscillators
//!   (heart, legs, respiration) with dynamic effort (`tau`) modulation.
//! - [`physiology`] — Foundational physiological signal processing (PPG,
//!   respiration, etc.).
//!
//! ## Features
//!
//! - Idiomatic Rust core with minimal dependencies
//! - Fixed-step RK4 integration (sourced from `symworx-math`)
//! - Full PyO3 bindings for Python interop
//! - Standalone Python package via `maturin` (`import symworx_biosym`)
//! - Also available under the unified `symworx.biosym` namespace
//!
//! ## Quick Start (Rust)
//!
//! ```rust
//! use symworx_biosym::{GaitParams, SymCpgModel};
//!
//! let params = GaitParams::with_defaults();
//! let mut model = SymCpgModel::new(None); // default config
//! let (times, states) = model.run(0.0, 60.0, 0.01);
//! ```
//!
//! Gait analysis: `GaitData` + `to_gait_stats` / calculate_* for intervals, cadence, lengths, symmetry.

// #![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/symworx-biosym")]

// Modules
/// Biomechanics modeling and utilities.
/// Re-exports gait parameters, data, `GaitStats`, `GaitAnalysis`, and
/// quality-aware stride detection (`detect_gait_strides*`, `analyze_gait*`).
pub mod biomechanics;

/// Central pattern generator using VdP
pub mod cpg;

/// Physiology modeling and utilities.
pub mod physiology;

/// Shared cross-domain primitives (e.g. `IntervalSeries` for event timing in PPG,
/// respiration, gait, etc., plus processing parameters).
/// Preferred path for new code: `common::*` (also re-exported at root and via
/// `physiology::common` for backward compatibility). This is the home for
/// cross-biomech and cross-physiology utilities so future domains (CMJ, pedaling,
/// etc.) can rely on them without creating direct dependencies between physiology
/// and biomechanics.
pub mod common;

// Re-exports
pub use biomechanics::*;
pub use cpg::*;
pub use physiology::*;

// Surface shared cross-domain items at the crate root for convenience
// (new code should prefer `symworx_biosym::common::*`).
pub use common::processing::{
    BandpassParams, PeakDetectionParams, PhysiologyProcessingParams, apply_bandpass,
    apply_peak_overrides,
};
pub use common::IntervalSeries;

// Compatibility shim so `crate::processing::...` and any old references continue to work.
// New code: use `crate::common::processing`.
pub mod processing {
    pub use crate::common::processing::*;
}

// Version info
/// Current version of the `symworx-biosym` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
