// core/src/io/mod.rs
// Copyright (C) 2026 cSYMd

pub mod csv;
pub mod gbd;
pub mod ibi;
pub mod parquet;
pub mod traits;

pub use csv::{CsvReader, CsvWriter};
pub use gbd::{GbdReader, GbdTable};
pub use ibi::{IbiRecord, read_ibi};
pub use parquet::ParquetReader;

use crate::errors::SymError;
use crate::io::traits::{SymReader, SymWriter};

// Parent load function
pub fn load_any(path: &str) -> Result<Vec<Vec<f64>>, SymError> {
    if path.ends_with(".csv") {
        CsvReader::read(path)
    } else if path.ends_with(".parquet") {
        ParquetReader::read(path)
    } else {
        Err(SymError::UnsupportedFormat(path.into()))
    }
}
