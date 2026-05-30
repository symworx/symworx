// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! # symworx-math
//!
//! Core mathematical utilities for the SymWorx ecosystem.
//!
//! This crate contains general-purpose numerical tools used across
//! `symworx-core` and domain crates (especially `symworx-biosym`).
//!
//! ## Notable Modules
//!
//! - [`series`] — Low-level operations on ordered sequences
//!   (the canonical home for successive differences and similar primitives).

#![allow(unused_imports)]
#![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/symworx-math")]

// Modules
/// Generate data specific to a defined distribution.
pub mod distributions;

/// Integration.
pub mod integration;

/// Oscillators and related functions.
pub mod oscillators;

/// Random numbers.
pub mod random;

/// Special functions (e.g., beta, gamma).  
pub mod special;

/// Series and sequential operations (differences, etc.).
pub mod series;

// Re-exports
pub use distributions::{beta_kernel, beta_pdf, gamma_kernel, gamma_pdf};
pub use integration::{cumtrapz, rk4_integrate, rk4_step, trapz};
pub use oscillators::VanDerPol;
pub use random::*;
pub use special::{beta, gamma, ln_beta, ln_gamma};
pub use series::{
    ewma, rolling_mean, rolling_std, successive_absolute_differences, successive_differences,
};

// Namespaced re-exports (for convenience)
/// Probability distributions: kernels, PDFs, sampling.
pub mod dist {
    pub use super::distributions::*;
}

/// Special mathematical functions (Gamma, Beta, etc.).
pub mod special_fn {
    pub use super::special::*;
}

/// Numerical integration methods (trapezoidal + ODE solvers).
pub mod integrate {
    pub use super::integration::*;
}

/// Series and sequential difference operations.
pub mod series_ops {
    pub use super::series::*;
}

// Version info
/// Current version of the `symworx-math` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
