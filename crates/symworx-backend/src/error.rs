// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

use thiserror::Error;

/// Backend error type for `symworx-backend`.
#[derive(Debug, Error)]
pub enum BackendError {
    /// Named supervised task failed or was rejected.
    #[error("task error: {0}")]
    Task(String),

    /// Shutdown coordination failed.
    #[error("shutdown error: {0}")]
    Shutdown(String),

    /// Configuration from the environment or caller was invalid.
    #[error("config error: {0}")]
    Config(String),

    /// Object-store operation failed.
    #[error("object store error: {0}")]
    Store(String),

    /// AWS / Azure I/O is configured but the SDK feature is not compiled in.
    #[error("{provider} object store is not compiled in ({hint})")]
    CloudDisabled {
        /// Provider name (`aws` or `azure`).
        provider: String,
        /// How to enable or what to use instead.
        hint: String,
    },

    /// Wrapped foreign error.
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl BackendError {
    pub(crate) fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    pub(crate) fn store(msg: impl Into<String>) -> Self {
        Self::Store(msg.into())
    }

    pub(crate) fn task(msg: impl Into<String>) -> Self {
        Self::Task(msg.into())
    }
}
