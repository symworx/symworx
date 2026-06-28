// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

use symworx_error::SymError;

/// A stub implementation of a GBD reader that simulates querying a GBD file with SQL.
pub struct GbdReader;

impl GbdReader {
    /// Run the SQL query against GBD file (stub).
    pub fn query(_path: &str, sql: &str) -> Result<GbdTable, SymError> {
        // Stub: return the SQL string as a fake table
        Ok(GbdTable {
            name: "query_result".to_string(),
            rows: vec![vec!["sql".into(), sql.into()]],
        })
    }
}

/// Result table from GBD query.
pub struct GbdTable {
    /// Name of the table.
    pub name: String,
    /// Number of rows in the table.
    pub rows: Vec<Vec<String>>,
}
