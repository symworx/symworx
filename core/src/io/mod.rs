#![allow(unused_imports)]
#![allow(dead_code)]

// core/src/io/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

pub mod csv;
pub mod parquet;
// pub mod edf;
pub mod traits;

pub use csv::CsvReader;
pub use parquet::ParquetReader;
// pub use edf::EdfReader;

use crate::errors::SymError;
use crate::io::traits::SymReader;

pub fn load_any(path: &str) -> Result<Vec<Vec<f64>>, SymError> {
    if path.ends_with(".csv") {
        CsvReader::read(path)
    } else if path.ends_with(".parquet") {
        ParquetReader::read(path)
    // ------------------------------------------------------
    // Commenting out edf for now...
    // 
    // } else if path.ends_with(".edf") {
    //     EdfReader::read(path)
    // ------------------------------------------------------
    } else {
        Err(SymError::UnsupportedFormat(path.into()))
    }
}
