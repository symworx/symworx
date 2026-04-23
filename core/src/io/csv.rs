#![allow(unused_imports)]
#![allow(dead_code)]

// core/src/io/csv.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use crate::errors::SymError;
use crate::io::traits::SymReader;
use ::csv::ReaderBuilder;

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
                .map(|v| v.parse::<f64>())
                .collect::<Result<Vec<_>, _>>()?;
            rows.push(row);
        }

        Ok(rows)
    }
}
