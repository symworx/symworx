// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Tabular numeric data for StatsSym / general analysis (CSV first).
//!
//! Loads headered CSV into column-oriented storage. Non-numeric columns are
//! recorded as skipped so the TUI can show names without forcing parse failure.

use std::{
    fs::File,
    path::Path,
};

use csv::{
    ReaderBuilder,
    WriterBuilder,
};
use symworx_error::SymError;

/// In-memory numeric table (column-major) for statistical workflows.
#[derive(Debug, Clone, Default)]
pub struct TableData {
    /// Source path or synthetic label.
    pub source: String,
    /// Column names (aligned with [`Self::columns`]).
    pub headers: Vec<String>,
    /// Numeric columns; each inner vec is one full column (`n_rows` long).
    pub columns: Vec<Vec<f64>>,
    /// Headers present in the file but not parsed as numeric.
    pub skipped_headers: Vec<String>,
}

impl TableData {
    /// Number of rows (0 if no columns).
    pub fn n_rows(&self) -> usize {
        self.columns.first().map(|c| c.len()).unwrap_or(0)
    }

    /// Number of numeric columns.
    pub fn n_cols(&self) -> usize {
        self.columns.len()
    }

    /// True when there are no rows or no numeric columns.
    pub fn is_empty(&self) -> bool {
        self.n_rows() == 0 || self.n_cols() == 0
    }

    /// Row-major copy (for APIs that expect `Vec<Vec<f64>>` rows).
    pub fn to_row_major(&self) -> Vec<Vec<f64>> {
        let n = self.n_rows();
        let p = self.n_cols();
        let mut rows = Vec::with_capacity(n);
        for i in 0..n {
            let mut row = Vec::with_capacity(p);
            for c in 0..p {
                row.push(self.columns[c][i]);
            }
            rows.push(row);
        }
        rows
    }

    /// Column index by case-insensitive header name.
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.headers.iter().position(|h| h.eq_ignore_ascii_case(name))
    }
}

/// Load a headered CSV as a numeric table.
///
/// - First row = headers.
/// - A column is kept if **every** non-empty cell parses as `f64`.
/// - Empty cells become `0.0` only if the column is otherwise numeric (optional
///   strict mode later). Currently empty → treat as non-numeric column skip.
/// - Columns that fail numeric parse are listed in [`TableData::skipped_headers`].
pub fn load_numeric_table(path: &str) -> Result<TableData, SymError> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)
        .map_err(|e| SymError::UnsupportedFormat(format!("csv open: {e}")))?;

    let headers: Vec<String> = rdr
        .headers()
        .map_err(|e| SymError::UnsupportedFormat(format!("csv headers: {e}")))?
        .iter()
        .map(|s| s.trim().to_string())
        .collect();

    if headers.is_empty() {
        return Err(SymError::UnsupportedFormat("csv has no headers".into()));
    }

    let ncols = headers.len();
    let mut col_ok = vec![true; ncols];
    let mut col_data: Vec<Vec<f64>> = vec![Vec::new(); ncols];

    for result in rdr.records() {
        let rec = result.map_err(|e| SymError::UnsupportedFormat(format!("csv row: {e}")))?;
        for c in 0..ncols {
            if !col_ok[c] {
                continue;
            }
            let cell = rec.get(c).map(|s| s.trim()).unwrap_or("");
            if cell.is_empty() {
                // empty cell: fill 0.0 for numeric columns (common for sparse tables)
                col_data[c].push(0.0);
            } else if let Ok(v) = cell.parse::<f64>() {
                col_data[c].push(v);
            } else {
                col_ok[c] = false;
                col_data[c].clear();
            }
        }
    }

    let mut out_headers = Vec::new();
    let mut out_cols = Vec::new();
    let mut skipped = Vec::new();
    for c in 0..ncols {
        if col_ok[c] && !col_data[c].is_empty() {
            out_headers.push(headers[c].clone());
            out_cols.push(std::mem::take(&mut col_data[c]));
        } else {
            skipped.push(headers[c].clone());
        }
    }

    if out_cols.is_empty() {
        return Err(SymError::UnsupportedFormat("no numeric columns found in csv".into()));
    }

    // Align lengths (ragged flexible CSV)
    let n = out_cols.iter().map(|c| c.len()).min().unwrap_or(0);
    for col in &mut out_cols {
        col.truncate(n);
    }

    Ok(TableData {
        source: path.to_string(),
        headers: out_headers,
        columns: out_cols,
        skipped_headers: skipped,
    })
}

/// Write a numeric table to CSV with headers.
pub fn write_numeric_table(path: &str, table: &TableData) -> Result<(), SymError> {
    if table.is_empty() {
        return Err(SymError::UnsupportedFormat("empty table".into()));
    }
    let file = File::create(Path::new(path)).map_err(SymError::Io)?;
    let mut wtr = WriterBuilder::new().from_writer(file);
    wtr.write_record(&table.headers)
        .map_err(|e| SymError::Io(std::io::Error::other(e.to_string())))?;
    let n = table.n_rows();
    let p = table.n_cols();
    for i in 0..n {
        let mut rec = Vec::with_capacity(p);
        for c in 0..p {
            rec.push(format!("{}", table.columns[c][i]));
        }
        wtr.write_record(&rec)
            .map_err(|e| SymError::Io(std::io::Error::other(e.to_string())))?;
    }
    wtr.flush().map_err(SymError::Io)?;
    Ok(())
}

/// Convenience: write headers + columns (column-major) to CSV.
pub fn write_columns_csv(path: &str, headers: &[String], columns: &[Vec<f64>]) -> Result<(), SymError> {
    write_numeric_table(
        path,
        &TableData {
            source: path.to_string(),
            headers: headers.to_vec(),
            columns: columns.to_vec(),
            skipped_headers: Vec::new(),
        },
    )
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn load_simple_numeric_csv() {
        let dir = std::env::temp_dir().join(format!("symworx_table_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("t.csv");
        {
            let mut f = File::create(&path).unwrap();
            writeln!(f, "x,y,label").unwrap();
            writeln!(f, "1.0,2.0,a").unwrap();
            writeln!(f, "3.0,4.0,b").unwrap();
        }
        let t = load_numeric_table(path.to_str().unwrap()).unwrap();
        assert_eq!(t.n_cols(), 2);
        assert_eq!(t.n_rows(), 2);
        assert_eq!(t.headers, vec!["x", "y"]);
        assert!(t.skipped_headers.iter().any(|h| h == "label"));
        assert_eq!(t.columns[0], vec![1.0, 3.0]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
