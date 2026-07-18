// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Error types for host-side embed streaming.

use thiserror::Error;

/// Errors from protocol parsing, stream sources, and buffers.
#[derive(Error, Debug)]
pub enum EmbedError {
    /// JSON line could not be parsed or required fields were invalid.
    #[error("protocol parse error: {0}")]
    Protocol(String),

    /// I/O failure (serial port, etc.).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serial port open/config failure.
    #[error("serial error: {0}")]
    Serial(String),

    /// Invalid configuration (empty port, zero capacity, etc.).
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),
}

/// Convenience result alias.
pub type Result<T> = std::result::Result<T, EmbedError>;
