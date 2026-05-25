// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

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

// Public API
pub mod filters;
pub mod processing;

// Re-exports
pub use filters::*;
pub use processing::*;

// Version info
/// Current version of the `symworx-signal` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
