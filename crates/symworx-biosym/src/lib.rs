// symworx/crates/symworx-biosym/src/lib.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

//! # symworx-biosym
//!
//! Biological signals, biomechanics, and coupled physiological modeling crate
//! within the SymWorx ecosystem.
//!
//! ## Core Modules
//!
//! - [`biomechanics`] — Gait analysis, spatiotemporal metrics, stride/step
//!   length, symmetry, and related data models.
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

#![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/symworx-biosym")]

// ==========================================================
// Modules
// ==========================================================
/// Biomechanics modeling and utilities (Gait analysis, spatiotemporal metrics, stride/step).
pub mod biomechanics;

/// Central pattern generator using VdP
pub mod cpg;

/// Physiology modeling and utilities.
pub mod physiology;

// ==========================================================
// Re-exports
// ==========================================================
pub use biomechanics::*;
pub use cpg::*;
pub use physiology::*;

// ==========================================================
// Version info
// ==========================================================
/// Current version of the `symworx-biosym` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

