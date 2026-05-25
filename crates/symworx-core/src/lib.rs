// Copyright (C) 2026 cSYMd, All rights reserved.

//! # symworx-core
//!
//! The foundational crate of the SymWorx ecosystem.
//!
//! This crate re-exports and aggregates the low-level utilities used across
//! all other SymWorx crates (math, signal processing, I/O, statistics,
//! dynamics, error handling, etc.).
//!
//! ## What it provides
//!
//! - **Re-exports** of core sub-crates for convenient access:
//!   - [`math`] — Numerical primitives, RK4 integration, linear algebra helpers
//!   - [`signal`] — Filtering, peak detection, event finding
//!   - [`stats`] — Basic and advanced statistical tools
//!   - [`dynamics`] — Nonlinear dynamics, oscillators, integration support
//!   - [`io`] — I/O traits and common readers/writers (CSV, Parquet, etc.)
//!   - [`error`] — Unified error types (`SymError`)
//!   - [`backend`] — Backend utilities and abstractions
//!
//! ## Usage
//!
//! ```rust
//! use symworx_core::{math, signal, stats};
//!
//! // Example: integrate with RK4 from math
//! let (times, states) = math::integrate::rk4_integrate(...);
//!
//! // Peak detection
//! let peaks = signal::processing::find_peaks(&signal_data, ...);
//! ```
//!
//! Most users will import this crate and get everything they need via the re-exports.
//! Domain-specific crates (`symworx-biosym`, `symworx-loadsym`, etc.) depend on this one.

#![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/symworx-core")]

// ==========================================================
// Re-exports
// ==========================================================
pub use symworx_error as error;
pub use symworx_error::SymError;

// Main modules
pub use symworx_math as math;
pub use symworx_io as io;
pub use symworx_signal as signal;
pub use symworx_stats as stats;
pub use symworx_dynamics as dynamics;
pub use symworx_backend as backend;

// Re-export of commonly used items
pub use symworx_signal::processing::{Peak, PeakDetect, PeakFinderBuilder};
pub use symworx_signal::filters;
pub use symworx_io::traits::*;
pub use symworx_stats::basic::*;

// ==========================================================
// Version info
// ==========================================================
/// Current version of the `symworx-core` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
