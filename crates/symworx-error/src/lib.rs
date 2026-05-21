// symworx/crates/symworx-error/src/lib.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

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

// ==========================================================
// Public API
// ==========================================================
pub mod error;
pub use error::*;

// ==========================================================
// Version info
// ==========================================================
/// Current version of the `symworx-error` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
