// core/src/io/csv.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use crate::errors::SymError;
use crate::io::traits::{SymReader, SymWriter};
use ::csv::{ReaderBuilder, WriterBuilder};
use std::fs::File;

// ===========================================================
// CSV Reader & Writer
// ===========================================================
// Reader
// -----------------------------------------------------------
pub struct CsvReader;

impl SymReader for CsvReader {
    type Output = Vec<Vec<f64>>;

    fn read(path: &str) -> Result<Self::Output, SymError> {
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .from_path(path)?; // -> SymError::Csv

        let mut rows = Vec::new();

        for result in rdr.records() {
            let record = result?; // -> SymError::Csv

            let row = record
                .iter()
                .map(|v| v.parse::<f64>().map_err(SymError::ParseFloat))
                .collect::<Result<Vec<_>, _>>()?;

            rows.push(row);
        }

        Ok(rows)
    }
}

// -----------------------------------------------------------
// Writer
// -----------------------------------------------------------
pub struct CsvWriter;

impl SymWriter for CsvWriter {
    type Input = Vec<Vec<f64>>;

    fn write(path: &str, data: &Self::Input) -> Result<(), SymError> {
        let file = File::create(path).map_err(SymError::Io)?;
        let mut wtr = WriterBuilder::new().from_writer(file);

        for row in data {
            wtr.serialize(row)?; // -> SymError::Csv
        }

        wtr.flush().map_err(SymError::Io)
    }
}
