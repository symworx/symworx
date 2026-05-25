// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

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
