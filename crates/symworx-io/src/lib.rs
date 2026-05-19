// core/src/io/mod.rs
// Copyright (C) 2026 cSYMd

pub mod csv;
pub mod gbd;
pub mod ibi;
pub mod parquet;
pub mod traits;

use symworx_error::SymError;

pub use csv::{CsvReader, CsvWriter};
pub use gbd::{GbdReader, GbdTable};
pub use ibi::{IbiRecord, read_ibi};
pub use parquet::ParquetReader;

use traits::{SymReader, SymWriter};

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
