// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

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