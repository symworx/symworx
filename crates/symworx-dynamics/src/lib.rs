// symworx/crates/symworx-dynamics/src/lib.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

//! # symworx-dynamics
//!
//! Nonlinear dynamics, chaos, and recurrence analysis tools for SymWorx.
//!
//! This crate provides utilities for analyzing complex physiological and
//! biomechanical time series using techniques from nonlinear dynamical systems.
//!
//! ## Modules
//!
//! - [`embedding`] — Phase space reconstruction (`edim`, false nearest neighbors)
//! - [`entropy`] — Sample entropy and related complexity measures
//! - [`rqa`] — Recurrence Quantification Analysis (RQA)
//!
//! These tools are especially useful for gait, heart rate, respiration, and
//! other quasi-periodic biological signals.

#![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/symworx-dynamics")]

// ==========================================================
// Public API
// ==========================================================
pub mod embedding;
pub mod entropy;
pub mod rqa;

pub use embedding::{edim, fnn};
pub use entropy::{sample_entropy};

// ==========================================================
// Version info
// ==========================================================
/// Current version of the `symworx-dynamics` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");;
