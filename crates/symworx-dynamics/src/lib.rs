// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

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

// Modules
/// Embedding dimension calculations (e.g., edim, fnn).
pub mod embedding;
/// Entropy measures (e.g., sample entropy, others tbd).
pub mod entropy;
/// Recurrence and cross-recurrence calculations.
pub mod rqa;

// Re-exports
pub use embedding::{
    edim,
    fnn,
};
pub use entropy::sample_entropy;
pub use rqa::{
    DEFAULT_LMIN,
    DEFAULT_VMIN,
    RecurrencePlot,
    RqaResult,
    rqa,
    rqa_from_trajectory,
};

// Version info
/// Current version of the `symworx-dynamics` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
