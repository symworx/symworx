// core/src/io/gbd.rs
// Copyright (C) 2026 cSYMd, All rights reserved.
//
// --- Example Usage ----------------------------------------
// use core::io::gbd::GbdReader;
// use core::io::traits::SymReader;

// fn main() -> Result<(), core::errors::SymError> {
//     let table = GbdReader::read("fake.gdb")?;

//     println!("Table: {}", table.name);
//     println!("Rows: {:?}", table.rows);

//     Ok(())
// }
//
// ----------------------------------------------------------

use crate::errors::SymError;
use crate::io::traits::SymReader;

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
