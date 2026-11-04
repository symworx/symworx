// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

use std::{
    fs::File,
    sync::Arc,
};

use parquet::{
    basic::{
        LogicalType,
        Repetition,
        Type as PhysicalType,
    },
    data_type::{
        ByteArray,
        ByteArrayType,
    },
    file::{
        reader::{
            FileReader,
            SerializedFileReader,
        },
        writer::SerializedFileWriter,
    },
    record::Field,
    schema::types::Type,
};
use symworx_error::SymError;

use crate::traits::SymReader;

/// Parquet Reader implementation for `symworx-io`.
///
/// Supports reading numeric Parquet files.
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

/// Write a header and string records to Parquet.
///
/// Each header becomes a required UTF8 column, and each value stays text.
/// Pages use the parquet crate's default uncompressed codec. [`ParquetReader`]
/// only keeps numeric columns, so read this file through the parquet record API.
pub fn write_records_parquet(path: &str, headers: &[String], rows: &[Vec<String>]) -> Result<(), SymError> {
    if headers.is_empty() {
        return Err(SymError::UnsupportedFormat("parquet headers are empty".into()));
    }
    for (index, name) in headers.iter().enumerate() {
        if headers[..index].iter().any(|earlier| earlier == name) {
            return Err(SymError::UnsupportedFormat(format!(
                "parquet header {name:?} is repeated"
            )));
        }
    }
    for (index, row) in rows.iter().enumerate() {
        if row.len() != headers.len() {
            return Err(SymError::UnsupportedFormat(format!(
                "parquet row {} has {} fields, headers have {}",
                index + 1,
                row.len(),
                headers.len()
            )));
        }
    }

    let schema = string_record_schema(headers)?;
    let file = File::create(path)?;
    let mut writer = SerializedFileWriter::new(file, schema, Default::default())?;
    if !rows.is_empty() {
        let columns = string_columns(rows);
        let mut row_group_writer = writer.next_row_group()?;
        for column in &columns {
            let mut col_writer = row_group_writer
                .next_column()?
                .ok_or_else(|| SymError::UnsupportedFormat("parquet column writer ended before the headers".into()))?;
            col_writer.typed::<ByteArrayType>().write_batch(column, None, None)?;
            col_writer.close()?;
        }
        row_group_writer.close()?;
    }
    writer.close()?;
    Ok(())
}

fn string_record_schema(headers: &[String]) -> Result<Arc<Type>, SymError> {
    let mut fields = Vec::with_capacity(headers.len());
    for name in headers {
        let field = Type::primitive_type_builder(name, PhysicalType::BYTE_ARRAY)
            .with_logical_type(Some(LogicalType::String))
            .with_repetition(Repetition::REQUIRED)
            .build()?;
        fields.push(Arc::new(field));
    }
    let schema = Type::group_type_builder("schema").with_fields(fields).build()?;
    Ok(Arc::new(schema))
}

fn string_columns(rows: &[Vec<String>]) -> Vec<Vec<ByteArray>> {
    let width = rows.first().map(Vec::len).unwrap_or(0);
    let mut columns = vec![Vec::with_capacity(rows.len()); width];
    for row in rows {
        for (column, value) in columns.iter_mut().zip(row.iter()) {
            column.push(ByteArray::from(value.as_str()));
        }
    }
    columns
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_records_keeps_text() {
        let path = std::env::temp_dir().join(format!("symworx_records_{}.parquet", std::process::id()));
        let headers = vec!["note".to_string(), "amount".to_string()];
        let rows = vec![vec!["AT&T, \"bill\"".to_string(), "-80.84".to_string()]];
        write_records_parquet(path.to_str().unwrap(), &headers, &rows).unwrap();

        let file = File::open(&path).unwrap();
        let reader = SerializedFileReader::new(file).unwrap();
        let mut iter = reader.get_row_iter(None).unwrap();
        let row = iter.next().unwrap().unwrap();
        let cols: Vec<_> = row.get_column_iter().collect();
        assert_eq!(cols[0].0, "note");
        assert_eq!(cols[0].1, &Field::Str("AT&T, \"bill\"".to_string()));
        assert_eq!(cols[1].0, "amount");
        assert_eq!(cols[1].1, &Field::Str("-80.84".to_string()));
        assert!(iter.next().is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn write_records_rejects_a_short_row() {
        let path = std::env::temp_dir().join(format!("symworx_records_bad_{}.parquet", std::process::id()));
        std::fs::write(&path, b"keep").unwrap();
        let headers = vec!["a".to_string(), "b".to_string()];
        let rows = vec![vec!["only".to_string()]];
        let err = write_records_parquet(path.to_str().unwrap(), &headers, &rows).unwrap_err();
        assert!(err.to_string().contains("parquet row 1"));
        assert_eq!(std::fs::read(&path).unwrap(), b"keep");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn write_records_rejects_a_repeated_header() {
        let path = std::env::temp_dir().join(format!("symworx_records_dup_{}.parquet", std::process::id()));
        std::fs::write(&path, b"keep").unwrap();
        let headers = vec!["a".to_string(), "a".to_string()];
        let err = write_records_parquet(path.to_str().unwrap(), &headers, &[]).unwrap_err();
        assert!(err.to_string().contains("repeated"));
        assert_eq!(std::fs::read(&path).unwrap(), b"keep");
        let _ = std::fs::remove_file(&path);
    }
}
