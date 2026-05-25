// Copyright (C) 2026 cSYMd, All rights reserved.

use std::fmt;
use std::io;

#[derive(Debug)]
pub enum SymError {
    Io(io::Error),
    Csv(::csv::Error),
    Parquet(::parquet::errors::ParquetError),
    // Edf(::edf::Error),
    ParseFloat(std::num::ParseFloatError),
    UnsupportedFormat(String),
    // UnsupportedFormat(String),
}

impl fmt::Display for SymError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymError::Io(e) => write!(f, "IO error: {}", e),
            SymError::Csv(e) => write!(f, "CSV error: {}", e),
            SymError::Parquet(e) => write!(f, "Parquet error: {}", e),
            // SymError::Edf(e) => write!(f, "EDF error: {}", e),
            SymError::ParseFloat(e) => write!(f, "Parse float error: {}", e),
            SymError::UnsupportedFormat(ext) => {
                write!(f, "Unsupported file format: {}", ext)
            }
        }
    }
}

impl std::error::Error for SymError {}

impl From<io::Error> for SymError {
    fn from(e: io::Error) -> Self {
        SymError::Io(e)
    }
}

impl From<::csv::Error> for SymError {
    fn from(e: ::csv::Error) -> Self {
        SymError::Csv(e)
    }
}

impl From<::parquet::errors::ParquetError> for SymError {
    fn from(e: ::parquet::errors::ParquetError) -> Self {
        SymError::Parquet(e)
    }
}

// impl From<::edf::Error> for SymError {
//     fn from(e: ::edf::Error) -> Self {
//         SymError::Edf(e)
//     }
// }

impl From<std::num::ParseFloatError> for SymError {
    fn from(e: std::num::ParseFloatError) -> Self {
        SymError::ParseFloat(e)
    }
}
