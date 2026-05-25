// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Backend utilities for symworx.
//!
//! This crate provides common infrastructure for server lifecycle management,
//! process supervision, and related backend services.
//!
//! It is intended to be used by higher-level crates such as `symworx-core`
//! and application servers.

#![warn(missing_docs)]

/// Proecess management utilities
pub mod process_manager;

/// Web server/API background support
pub mod server;

/// Crate version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
