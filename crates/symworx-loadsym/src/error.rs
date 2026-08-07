// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Error types for the loadsym crate.
//!
//! Domain-specific errors for load calculations (validation, insufficient data, etc.).
//! These are distinct from low-level IO errors in `symworx-error`.

use thiserror::Error;

/// Errors that can occur during load quantification, ACWR, readiness, or nutrition modeling.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum LoadSymError {
    /// Input arrays had incompatible lengths or shapes.
    #[error("input length mismatch: {0}")]
    LengthMismatch(String),

    /// Not enough observations to compute a stable metric (e.g. ACWR with min window).
    #[error("insufficient data: {0}")]
    InsufficientData(String),

    /// Invalid parameter (negative window, zero span, out-of-range age, etc.).
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),

    /// Value out of physiological or logical range (e.g. negative load after validation).
    #[error("invalid value: {0}")]
    InvalidValue(String),
}

/// Convenience Result alias for loadsym operations.
pub type Result<T> = std::result::Result<T, LoadSymError>;
