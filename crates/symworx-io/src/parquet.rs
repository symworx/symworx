// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::record::Field;
use std::fs::File;

use crate::traits::SymReader;
use symworx_error::SymError;

/// Read in Parquet files.
pub struct ParquetReader;

impl SymReader for ParquetReader {
    type Output = Vec<Vec<f64>>;

    fn read(path: &str) -> Result<Self::Output, SymError> {
        let file = File::open(path)?;
        let reader = SerializedFileReader::new(file)?;
        let iter = reader.get_row_iter(None)?;

        let mut rows = Vec::new();

        for row in iter {
            let row = row?;

            let mut r: Vec<f64> = Vec::new();

            for (_, v) in row.get_column_iter() {
                let val: f64 = match v {
                    Field::Double(x) => *x,
                    Field::Float(x) => *x as f64,
                    Field::Int(x) => *x as f64,
                    Field::Long(x) => *x as f64,
                    _ => 0.0,
                };
                r.push(val);
            }

            rows.push(r);
        }

        Ok(rows)
    }
}
