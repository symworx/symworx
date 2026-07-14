// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! # symworx-loadsym
//!
//! Training load quantification, optimization, nutrition, and energy expenditure
//! modeling for the SymWorx ecosystem.
//!
//! ## Capabilities
//! - Mechanical & physiological load helpers (ride metrics, NP/TSS, etc.)
//! - **ACWR / EWMA / risk classification** for daily load series
//! - **Pulse-response (fitness–fatigue)** — Banister and PMC-style CTL/ATL/TSB
//!   (`load::pulse_response`) with open-loop recovery forecast
//! - **Multi-day load planning** — overload / maintenance / recovery goals
//!   (`load::optimization`)
//! - Monotony and strain
//! - Nutrition & body-composition modeling (BMR, TDEE, weight-loss trajectories)
//!
//! Optional `sqlite` catalog + `symload` CLI live under feature flags (see crate README).

// #![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/symworx-loadsym")]

// Modules
pub mod error;
pub mod load;
pub mod nutrition;

/// Personal SQLite catalog (init / ingest). Enabled with the `sqlite` feature.
/// Data files must live outside this repository (e.g. under `$VELOFIT_HOME/db/`).
#[cfg(feature = "sqlite")]
pub mod catalog;

// Re-exports
pub use error::{
    LoadSymError,
    Result,
};
pub use load::*;
pub use nutrition::*;

// Version info
/// Current version of the `symworx-loadsym` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
