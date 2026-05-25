// Copyright (C) 2026 cSYMd, All rights reserved.

//! # symworx-loadsym
//!
//! Training load quantification, optimization, nutrition, and energy expenditure
//! modeling for the SymWorx ecosystem.
//!
//! This crate provides tools for calculating acute:chronic workload ratios,
//! training impulse (TRIMP), recovery modeling, nutritional macros, and
//! energy balance in athletic and clinical contexts.

#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(improper_ctypes_definitions)]
#![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/symworx-loadsym")]

// ==========================================================
// Modules
// ==========================================================
pub mod load;
pub mod nutrition;

// ==========================================================
// Re-exports
// ==========================================================
pub use load::*;
pub use nutrition::*;

// ==========================================================
// Version info
// ==========================================================
/// Current version of the `symworx-loadsym` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
