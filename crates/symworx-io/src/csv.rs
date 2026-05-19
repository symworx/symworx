// symworx/crates/symworx-io/src/csv.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use std::fs::File;
use csv::{ReaderBuilder, WriterBuilder};

use symworx_error::SymError;

use crate::traits::{SymReader, SymWriter};


/// CSV Reader implementation for `symworx-io`.
///
/// Supports reading numeric CSV files (no headers) into `Vec<Vec<f64>>`.
pub struct CsvReader;

impl SymReader for CsvReader {
    type Output = Vec<Vec<f64>>;

    fn read(path: &str) -> Result<Self::Output, SymError> {
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .from_path(path)?; 

        let mut rows = Vec::new();

        for result in rdr.records() {
            let record = result?; 

            let row = record
                .iter()
                .map(|v| v.parse::<f64>().map_err(SymError::ParseFloat))
                .collect::<Result<Vec<_>, _>>()?;

            rows.push(row);
        }

        Ok(rows)
    }
}

/// CSV Writer implementation for `symworx-io`.
///
/// Writes 2D numeric data (`Vec<Vec<f64>>`) to a CSV file.
pub struct CsvWriter;

impl SymWriter for CsvWriter {
    type Input = Vec<Vec<f64>>;

    fn write(path: &str, data: &Self::Input) -> Result<(), SymError> {
        let file = File::create(path).map_err(SymError::Io)?;
        let mut wtr = WriterBuilder::new().from_writer(file);

        for row in data {
            wtr.serialize(row)?; 
        }

        wtr.flush().map_err(SymError::Io)
    }
}
