// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! # symworx-io
//!
//! Core input/output functions and file handling.
//!
//! This crate contains general-purpose tools to read and write files
//! and is imported into both `symworx-core` and domain
//! crates (especially `symworx-biosym`).

#![warn(missing_docs)]

// Modules
/// CSV module and related utilities.
pub mod csv;

/// GBD module and related utilities.
pub mod gbd;

/// IBI module and related utilities.
pub mod ibi;

#[cfg(feature = "parquet")]
/// Parquet module and related utilities.
pub mod parquet;

/// Exercise / activity file support (FIT + other formats; sport-agnostic).
/// Enabled via `fit` feature (and future `gpx` etc).
pub mod activity;

/// Email input support (IMAP fetching of .fit files, e.g. from SRM PC8 emails).
/// Enabled via the `email` feature.
#[cfg(feature = "email")]
pub mod email;

#[cfg(feature = "email")]
pub use email::fetch_srm_fit_attachments;

/// Additional traits used in io.
pub mod traits;

// Re-exports
pub use activity::{ActivityData, load_activity, load_activity_power_series};
pub use csv::{CsvReader, CsvWriter};
pub use gbd::{GbdReader, GbdTable};
pub use ibi::{IbiRecord, read_ibi};
#[cfg(feature = "parquet")]
pub use parquet::ParquetReader;
use symworx_error::SymError;
use traits::SymReader;

/// Parent load function.
///
/// Auto-detect the file format (csv, parquet) and read in the file.
pub fn load_any(path: &str) -> Result<Vec<Vec<f64>>, SymError> {
    if path.ends_with(".csv") {
        CsvReader::read(path)
    } else if path.ends_with(".parquet") {
        #[cfg(feature = "parquet")]
        return ParquetReader::read(path);
        #[cfg(not(feature = "parquet"))]
        return Err(SymError::UnsupportedFormat(path.into()));
    } else {
        Err(SymError::UnsupportedFormat(path.into()))
    }
}

/// symworx-io version information.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
