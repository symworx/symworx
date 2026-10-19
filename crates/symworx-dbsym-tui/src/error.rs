// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Errors for the read-only catalog browser.

use thiserror::Error;

/// Failure opening a catalog or reading a page.
#[derive(Debug, Error)]
pub enum Error {
    /// Bad argv, missing file, or a path that is not a database.
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),

    /// URL asked for TLS. This build connects with `NoTls` only.
    #[error("TLS is not built into symdb-view ({0})")]
    TlsRequired(String),

    /// Filesystem failure outside the driver.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// SQLite driver failure.
    #[cfg(feature = "sqlite")]
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// Postgres driver failure.
    #[cfg(feature = "postgres")]
    #[error("postgres: {0}")]
    Postgres(#[from] postgres::Error),
}

/// Result alias for the browser.
pub type Result<T> = std::result::Result<T, Error>;
