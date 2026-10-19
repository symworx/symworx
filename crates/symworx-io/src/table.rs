// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Tabular numeric data for StatsSym / general analysis (CSV first).
//!
//! Headers come from the first row **or** from an explicit name list. Every
//! subsequent cell in a kept column is parsed as `f64`.

use std::{
    fs::File,
    io::{
        BufRead,
        BufReader,
    },
    path::Path,
};

use csv::{
    ReaderBuilder,
    WriterBuilder,
};
use symworx_error::SymError;

/// Field separator for [`load_numeric_table`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableDelimiter {
    /// `,`
    Comma,
    /// Tab
    Tab,
    /// Any run of ASCII whitespace.
    Whitespace,
}

impl TableDelimiter {
    /// Parse `"comma"` / `","`, `"tab"`, `"whitespace"` / `"space"`.
    pub fn parse(s: &str) -> Result<Self, SymError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "comma" | "," | "csv" => Ok(Self::Comma),
            "tab" | "\t" | "tsv" => Ok(Self::Tab),
            "whitespace" | "space" | "ws" => Ok(Self::Whitespace),
            other => Err(SymError::UnsupportedFormat(format!(
                "unknown table delimiter {other:?}"
            ))),
        }
    }
}

/// How to read a numeric table: delimiter, header row, optional column names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableReadOptions {
    /// Field separator.
    pub delimiter: TableDelimiter,
    /// If true, the first non-empty row is headers.
    pub has_headers: bool,
    /// Explicit column names. Without a header row these label columns in
    /// order (`c0`, `c1`, … if omitted). With a header row they select and
    /// rename matching columns (case-insensitive).
    pub names: Option<Vec<String>>,
}

impl Default for TableReadOptions {
    fn default() -> Self {
        Self {
            delimiter: TableDelimiter::Comma,
            has_headers: true,
            names: None,
        }
    }
}

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

    /// Values for a named column.
    pub fn column(&self, name: &str) -> Option<&[f64]> {
        let i = self.column_index(name)?;
        Some(self.columns[i].as_slice())
    }
}

/// Load a numeric table. Default `opts`: comma, first row is headers.
///
/// After names are known, every subsequent cell in a kept column is `f64`.
/// Other columns are skipped (`skipped_headers`); they are not stored as text.
/// Empty lines are skipped.
pub fn load_numeric_table(path: &str, opts: &TableReadOptions) -> Result<TableData, SymError> {
    let raw_rows = read_raw_rows(path, opts.delimiter)?;
    if raw_rows.is_empty() {
        return Err(SymError::UnsupportedFormat("table is empty".into()));
    }

    let (file_headers, data_rows) = if opts.has_headers {
        let headers = raw_rows[0].clone();
        (headers, &raw_rows[1..])
    } else {
        let n = raw_rows.iter().map(|r| r.len()).max().unwrap_or(0);
        let headers = (0..n).map(|i| format!("c{i}")).collect();
        (headers, raw_rows.as_slice())
    };

    let ResolvedColumns {
        index: keep_idx,
        names: keep_names,
        skipped,
    } = resolve_columns(&file_headers, opts.has_headers, opts.names.as_deref())?;
    if keep_idx.is_empty() {
        return Err(SymError::UnsupportedFormat("no columns selected".into()));
    }

    let require_numeric = opts.names.is_some();
    let mut col_ok = vec![true; keep_idx.len()];
    let mut col_data: Vec<Vec<f64>> = vec![Vec::new(); keep_idx.len()];

    for (row_i, rec) in data_rows.iter().enumerate() {
        for (k, &src) in keep_idx.iter().enumerate() {
            if !col_ok[k] {
                continue;
            }
            let cell = rec.get(src).map(|s| s.trim()).unwrap_or("");
            if cell.is_empty() {
                if require_numeric {
                    return Err(SymError::UnsupportedFormat(format!(
                        "{path}:{} empty cell in column {}",
                        row_i + 1,
                        keep_names[k]
                    )));
                }
                col_data[k].push(0.0);
            } else if let Ok(v) = cell.parse::<f64>() {
                col_data[k].push(v);
            } else if require_numeric {
                return Err(SymError::UnsupportedFormat(format!(
                    "{path}:{} column {} is not numeric: {cell:?}",
                    row_i + 1,
                    keep_names[k]
                )));
            } else {
                col_ok[k] = false;
                col_data[k].clear();
            }
        }
    }

    let mut out_headers = Vec::new();
    let mut out_cols = Vec::new();
    let mut skipped_headers = skipped;
    for k in 0..keep_idx.len() {
        if col_ok[k] && !col_data[k].is_empty() {
            out_headers.push(keep_names[k].clone());
            out_cols.push(std::mem::take(&mut col_data[k]));
        } else {
            skipped_headers.push(keep_names[k].clone());
        }
    }

    if out_cols.is_empty() {
        return Err(SymError::UnsupportedFormat("no numeric columns found".into()));
    }

    let n = out_cols.iter().map(|c| c.len()).min().unwrap_or(0);
    for col in &mut out_cols {
        col.truncate(n);
    }

    Ok(TableData {
        source: path.to_string(),
        headers: out_headers,
        columns: out_cols,
        skipped_headers,
    })
}

struct ResolvedColumns {
    index: Vec<usize>,
    names: Vec<String>,
    skipped: Vec<String>,
}

fn resolve_columns(
    file_headers: &[String],
    has_headers: bool,
    names: Option<&[String]>,
) -> Result<ResolvedColumns, SymError> {
    match names {
        None => {
            let index: Vec<usize> = (0..file_headers.len()).collect();
            Ok(ResolvedColumns {
                index,
                names: file_headers.to_vec(),
                skipped: Vec::new(),
            })
        }
        Some(wanted) if !has_headers => {
            if wanted.len() > file_headers.len() {
                return Err(SymError::UnsupportedFormat(format!(
                    "got {} columns, {} names",
                    file_headers.len(),
                    wanted.len()
                )));
            }
            let index: Vec<usize> = (0..wanted.len()).collect();
            let skipped = file_headers[wanted.len()..].to_vec();
            Ok(ResolvedColumns {
                index,
                names: wanted.to_vec(),
                skipped,
            })
        }
        Some(wanted) => {
            let mut index = Vec::new();
            let mut keep_names = Vec::new();
            for name in wanted {
                let Some(p) = file_headers.iter().position(|h| h.eq_ignore_ascii_case(name)) else {
                    return Err(SymError::UnsupportedFormat(format!(
                        "column {name:?} not in headers {file_headers:?}"
                    )));
                };
                index.push(p);
                keep_names.push(name.clone());
            }
            let skipped = file_headers
                .iter()
                .enumerate()
                .filter(|(i, _)| !index.contains(i))
                .map(|(_, h)| h.clone())
                .collect();
            Ok(ResolvedColumns {
                index,
                names: keep_names,
                skipped,
            })
        }
    }
}

fn read_raw_rows(path: &str, delimiter: TableDelimiter) -> Result<Vec<Vec<String>>, SymError> {
    match delimiter {
        TableDelimiter::Whitespace => read_whitespace_rows(path),
        TableDelimiter::Comma => read_csv_rows(path, b','),
        TableDelimiter::Tab => read_csv_rows(path, b'\t'),
    }
}

fn read_csv_rows(path: &str, delim: u8) -> Result<Vec<Vec<String>>, SymError> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .delimiter(delim)
        .from_path(path)
        .map_err(|e| SymError::UnsupportedFormat(format!("csv open: {e}")))?;
    let mut rows = Vec::new();
    for result in rdr.records() {
        let rec = result.map_err(|e| SymError::UnsupportedFormat(format!("csv row: {e}")))?;
        let row: Vec<String> = rec.iter().map(|s| s.trim().to_string()).collect();
        if row.iter().all(|s| s.is_empty()) {
            continue;
        }
        rows.push(row);
    }
    Ok(rows)
}

fn read_whitespace_rows(path: &str) -> Result<Vec<Vec<String>>, SymError> {
    let file = File::open(path).map_err(SymError::Io)?;
    let mut rows = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = line.map_err(SymError::Io)?;
        let parts: Vec<String> = line.split_whitespace().map(str::to_string).collect();
        if parts.is_empty() {
            continue;
        }
        rows.push(parts);
    }
    Ok(rows)
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

    fn tmp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("symworx_table_{}_{}", std::process::id(), name));
        let _ = std::fs::create_dir_all(&dir);
        dir.join(name)
    }

    #[test]
    fn load_simple_numeric_csv() {
        let path = tmp("t.csv");
        {
            let mut f = File::create(&path).unwrap();
            writeln!(f, "x,y,label").unwrap();
            writeln!(f, "1.0,2.0,a").unwrap();
            writeln!(f, "3.0,4.0,b").unwrap();
        }
        let t = load_numeric_table(path.to_str().unwrap(), &TableReadOptions::default()).unwrap();
        assert_eq!(t.n_cols(), 2);
        assert_eq!(t.n_rows(), 2);
        assert_eq!(t.headers, vec!["x", "y"]);
        assert!(t.skipped_headers.iter().any(|h| h == "label"));
        assert_eq!(t.columns[0], vec![1.0, 3.0]);
    }

    #[test]
    fn load_whitespace_with_explicit_names() {
        let path = tmp("rr.txt");
        {
            let mut f = File::create(&path).unwrap();
            writeln!(f, "0.000 1.304").unwrap();
            writeln!(f, "1.304 1.304").unwrap();
        }
        let opts = TableReadOptions {
            delimiter: TableDelimiter::Whitespace,
            has_headers: false,
            names: Some(vec!["t_s".into(), "rr_s".into()]),
        };
        let t = load_numeric_table(path.to_str().unwrap(), &opts).unwrap();
        assert_eq!(t.headers, vec!["t_s", "rr_s"]);
        assert_eq!(t.column("t_s").unwrap(), &[0.0, 1.304]);
        assert_eq!(t.column("rr_s").unwrap()[0], 1.304);
    }

    #[test]
    fn headered_select_by_name() {
        let path = tmp("sel.csv");
        {
            let mut f = File::create(&path).unwrap();
            writeln!(f, "time,rr,flag").unwrap();
            writeln!(f, "0,0.8,ok").unwrap();
            writeln!(f, "1,0.9,ok").unwrap();
        }
        let opts = TableReadOptions {
            delimiter: TableDelimiter::Comma,
            has_headers: true,
            names: Some(vec!["rr".into(), "time".into()]),
        };
        let t = load_numeric_table(path.to_str().unwrap(), &opts).unwrap();
        assert_eq!(t.headers, vec!["rr", "time"]);
        assert_eq!(t.column("rr").unwrap(), &[0.8, 0.9]);
    }

    #[test]
    fn explicit_names_reject_non_numeric() {
        let path = tmp("bad.csv");
        {
            let mut f = File::create(&path).unwrap();
            writeln!(f, "1.0,x").unwrap();
        }
        let opts = TableReadOptions {
            delimiter: TableDelimiter::Comma,
            has_headers: false,
            names: Some(vec!["a".into(), "b".into()]),
        };
        let err = load_numeric_table(path.to_str().unwrap(), &opts).unwrap_err();
        match err {
            SymError::UnsupportedFormat(s) => assert!(s.contains("not numeric"), "{s}"),
            other => panic!("{other:?}"),
        }
    }
}
