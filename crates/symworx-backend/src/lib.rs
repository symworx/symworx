// Copyright (C) 2026 cSYMd, All rights reserved.

//! Backend utilities for symworx.
//!
//! This crate provides common infrastructure for server lifecycle management,
//! process supervision, and related backend services.
//!
//! It is intended to be used by higher-level crates such as `symworx-core`
//! and application servers.

#![warn(missing_docs)]

// ==========================================================
// Modules
// ==========================================================
/// Proecess management utilities
pub mod process_manager;

/// Web server/API background support
pub mod server;

// ==========================================================
// Version info
// ==========================================================

/// Crate version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
