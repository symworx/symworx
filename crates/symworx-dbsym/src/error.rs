// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Catalog errors for dbSym.
//!
//! Domain failures (unknown preset, bad profile) live here — same `thiserror`
//! pattern as `LoadSymError` and `EmbedError`.
//!
//! `symworx_error::SymError` is the workspace I/O/CSV type, but that crate
//! always depends on `csv`, so this catalog does not take it on the default
//! path. Ingest can map `std::io::Error` / `csv::Error` at the boundary later.

use thiserror::Error;

/// Errors from init, seed, and (later) ingest/export.
#[derive(Error, Debug)]
pub enum DbSymError {
    /// Invalid argument (profile, path, dialect).
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),

    /// Seed preset name is not in the built-in list.
    #[error("unknown preset '{0}' (known: {1})")]
    UnknownPreset(String, String),

    /// Filesystem failure (create dir, write gitignore, copy object).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// SQLite driver failure.
    #[cfg(feature = "sqlite")]
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

/// Convenience result alias for dbSym operations.
pub type Result<T> = std::result::Result<T, DbSymError>;
