// symworx/crates/symworx-signal/src/lib.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

//! # symworx-signal
//!
//! Digital signal processing tools for physiological and biomechanical data
//! in the SymWorx ecosystem.
//!
//! This crate provides filtering, peak/event detection, and general signal
//! processing utilities optimized for noisy real-world biological time series
//! (PPG, ECG, accelerometry, force plates, etc.).

#![allow(unused_imports)]
#![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/symworx-signal")]

// ==========================================================
// Public API
// ==========================================================
/// Filters (e.g., Butterworth, Chebyshev, Kalman). 
pub mod filters;

/// Processing algorithms related to feature selection, interpolation, etc.
pub mod processing;

// ==========================================================
// Re-exports
// ==========================================================
pub use filters::*;
pub use processing::*;

// ==========================================================
// Version info
// ==========================================================
/// Current version of the `symworx-signal` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

