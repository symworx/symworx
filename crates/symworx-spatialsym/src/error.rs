// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Error types for the spatial crate.
//!
//! Domain-specific errors for trajectory handling, kinematics, and decision analysis.

use thiserror::Error;

/// Errors that can occur in spatial trajectory processing and analysis.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum SpatialError {
    /// Input arrays or slices had incompatible lengths or shapes.
    #[error("length/shape mismatch: {0}")]
    LengthMismatch(String),

    /// Not enough observations (e.g. < 2 points for differences, empty trajectory).
    #[error("insufficient data: {0}")]
    InsufficientData(String),

    /// Invalid parameter (zero dt, negative window, etc.).
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),

    /// Computation produced NaN or otherwise invalid numeric result.
    #[error("invalid numeric result: {0}")]
    InvalidValue(String),
}

/// Convenience Result alias for spatial operations.
pub type Result<T> = std::result::Result<T, SpatialError>;
