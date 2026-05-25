// Copyright (C) 2026 cSYMd, All rights reserved.

//! File conversion component for symview.
//! Uses symworx-io for .ibi + CSV, Polars for Parquet.

use anyhow::Result;
use polars::prelude::*;
use std::path::Path;

use symworx_io::{read_ibi, CsvWriter};

pub fn parquet_to_csv(input: &Path, output: &Path) -> Result<()> {
    let df = LazyFrame::scan_parquet(input, Default::default()).collect()?;
    let mut rows: Vec<Vec<f64>> = Vec::new();
    for i in 0..df.height() {
        let row: Vec<f64> = df.get_row(i).0
            .into_iter()
            .filter_map(|v| v.try_extract::<f64>().ok())
            .collect();
        if !row.is_empty() { rows.push(row); }
    }
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
    let out = output.map(|p| p.to_path_buf())
        .unwrap_or_else(|| {
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
