// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

#![cfg(feature = "supervision")]

use thiserror::Error;

/// Backend error type for symworx-backend.
#[derive(Debug, Error)]
pub enum BackendError {
    #[error("task error: {0}")]
    TaskError(String),

    #[error("shutdown error: {0}")]
    ShutdownError(String),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}
