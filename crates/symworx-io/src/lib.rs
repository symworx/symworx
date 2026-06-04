// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

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

/// Parquet module and related utilities.
pub mod parquet;

/// Additional traits used in io.
pub mod traits;

// Re-exports
pub use csv::{
    CsvReader,
    CsvWriter,
};
pub use gbd::{
    GbdReader,
    GbdTable,
};
pub use ibi::{
    IbiRecord,
    read_ibi,
};
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
        ParquetReader::read(path)
    } else {
        Err(SymError::UnsupportedFormat(path.into()))
    }
}

/// symworx-io version information.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
