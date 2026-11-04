// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

use std::{
    fs::File,
    path::Path,
};

use csv::{
    ReaderBuilder,
    WriterBuilder,
};
use symworx_error::SymError;

use crate::traits::{
    SymReader,
    SymWriter,
};

/// CSV Reader implementation for `symworx-io`.
///
/// Supports reading numeric CSV files (no headers) into `Vec<Vec<f64>>`.
pub struct CsvReader;

impl SymReader for CsvReader {
    type Output = Vec<Vec<f64>>;

    fn read(path: &str) -> Result<Self::Output, SymError> {
        let mut rdr = ReaderBuilder::new().has_headers(false).from_path(path)?;

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

/// Write a header and string records to CSV.
///
/// [`CsvWriter`] is headerless numeric data. This keeps text such as a payee,
/// a date, or a note. The `csv` crate quotes commas, quotes, and newlines.
pub fn write_records_csv(path: &str, headers: &[String], rows: &[Vec<String>]) -> Result<(), SymError> {
    if headers.is_empty() {
        return Err(SymError::UnsupportedFormat("csv headers are empty".into()));
    }
    for (index, row) in rows.iter().enumerate() {
        if row.len() != headers.len() {
            return Err(SymError::UnsupportedFormat(format!(
                "csv row {} has {} fields, headers have {}",
                index + 1,
                row.len(),
                headers.len()
            )));
        }
    }
    let file = File::create(Path::new(path))?;
    let mut wtr = WriterBuilder::new().from_writer(file);
    wtr.write_record(headers)?;
    for row in rows {
        wtr.write_record(row)?;
    }
    wtr.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_records_quotes_commas_and_quotes() {
        let path = std::env::temp_dir().join(format!("symworx_records_{}.csv", std::process::id()));
        let headers = vec!["note".to_string(), "amount".to_string()];
        let rows = vec![vec!["AT&T, \"bill\"".to_string(), "-80.84".to_string()]];
        write_records_csv(path.to_str().unwrap(), &headers, &rows).unwrap();
        let mut rdr = ReaderBuilder::new().has_headers(true).from_path(&path).unwrap();
        let headers: Vec<String> = rdr.headers().unwrap().iter().map(str::to_string).collect();
        assert_eq!(headers, vec!["note", "amount"]);
        let rec = rdr.records().next().unwrap().unwrap();
        assert_eq!(rec.get(0), Some("AT&T, \"bill\""));
        assert_eq!(rec.get(1), Some("-80.84"));
        assert!(rdr.records().next().is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn write_records_rejects_a_short_row() {
        let path = std::env::temp_dir().join(format!("symworx_records_bad_{}.csv", std::process::id()));
        std::fs::write(&path, b"keep\n").unwrap();
        let headers = vec!["a".to_string(), "b".to_string()];
        let rows = vec![vec!["only".to_string()]];
        let err = write_records_csv(path.to_str().unwrap(), &headers, &rows).unwrap_err();
        assert!(err.to_string().contains("csv row 1"));
        assert_eq!(std::fs::read(&path).unwrap(), b"keep\n");
        let _ = std::fs::remove_file(&path);
    }
}
