// Copyright (C) 2026 cSYMd, All rights reserved.

use symworx_error::SymError;

/// A stub implementation of a GBD reader that simulates querying a GBD file with SQL.
pub struct GbdReader;

pub struct GbdTable {
    pub name: String,
    pub rows: Vec<Vec<String>>,
}

impl GbdReader {
    pub fn query(_path: &str, sql: &str) -> Result<GbdTable, SymError> {
        // Stub: return the SQL string as a fake table
        Ok(GbdTable {
            name: "query_result".to_string(),
            rows: vec![
                vec!["sql".into(), sql.into()],
            ],
        })
    }
}
