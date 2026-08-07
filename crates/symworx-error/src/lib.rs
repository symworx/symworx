// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! # symworx-error
//!
//! Unified error handling for the entire SymWorx ecosystem.
//!
//! This crate defines the common [`SymError`] type and related error
//! variants used across all SymWorx crates (core, biosym, loadsym, etc.).
//!
//! It is built on `thiserror` and provides consistent, descriptive errors
//! with good support for PyO3 conversion.

#![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/symworx-error")]

// Modules
/// Error modules
pub mod error;
pub use error::*;

// Version info
/// Current version of the `symworx-error` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
