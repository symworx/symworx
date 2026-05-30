// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! # symworx-loadsym
//!
//! Training load quantification, optimization, nutrition, and energy expenditure
//! modeling for the SymWorx ecosystem.
//!
//! ## Core Capabilities (2026)
//! - Low-level mechanical & physiological load
//!   (expanding toward TRIMP family, sRPE, etc.).
//! - **ACWR / EWMA / risk classification** — the primary primitives for
//!   populating `player_load_metrics` (acute/chronic, acwr, ewma_*, risk_level).
//! - Monotony, strain, readiness, adaptive capacity, life-stress, and
//!   periodization recommendations
//! - Nutrition & body-composition modeling.
//!   (BMR, TDEE, deficit strategies, weight-loss trajectories)
//!

// #![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/symworx-loadsym")]

// Modules
pub mod error;
pub mod load;
pub mod nutrition;

// Re-exports
pub use error::{LoadSymError, Result};
pub use load::*;
pub use nutrition::*;

// Version info
/// Current version of the `symworx-loadsym` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
