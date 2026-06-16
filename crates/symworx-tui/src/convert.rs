// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! File conversion component for symview.
//!
//! All I/O goes through `symworx-io` (ParquetReader for .parquet when the
//! "parquet" feature is enabled on symworx-io — which the TUI enables by
//! default). Polars is never used for conversion or file I/O.

use std::path::Path;

use anyhow::Result;
use symworx_io::{
    read_ibi,
    traits::{
        SymReader,
        SymWriter,
    },
    CsvWriter,
    ParquetReader,
};

pub fn parquet_to_csv(input: &Path, output: &Path) -> Result<()> {
    // Use the controlled symworx-io ParquetReader (no polars/arrow stack here).
    let rows = ParquetReader::read(input.to_str().unwrap())?;
    CsvWriter::write(output.to_str().unwrap(), &rows)?;
    Ok(())
}

pub fn ibi_to_csv(input: &Path, output: &Path) -> Result<()> {
    let records = read_ibi(input.to_str().unwrap())?;
    let rows: Vec<Vec<f64>> = records
        .into_iter()
        .map(|r| vec![r.timestamp as f64, r.rr_ms as f64])
        .collect();
    CsvWriter::write(output.to_str().unwrap(), &rows)?;
    Ok(())
}

pub fn convert_to_csv(input: &Path, output: Option<&Path>) -> Result<()> {
    let out = output.map(|p| p.to_path_buf()).unwrap_or_else(|| {
        let mut p = input.to_path_buf();
        p.set_extension("csv");
        p
    });

    match input.extension().and_then(|e| e.to_str()) {
        Some("parquet") => parquet_to_csv(input, &out),
        Some("ibi") | Some("biosym") => ibi_to_csv(input, &out),
        _ => anyhow::bail!("Unsupported file type"),
    }
}
