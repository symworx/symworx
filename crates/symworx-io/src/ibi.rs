// symworx/crates/symworx-io/src/ibi.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use std::fs::File;
use std::io::{Read, BufReader};

use symworx_error::SymError;

/// IBI data record structure.
#[derive(Debug, Clone)]
pub struct IbiRecord {
    pub timestamp: u32,
    pub rr_ms: u16,
}

/// Read an IBI file into a single vector.
pub fn read_ibi(path: &str) -> Result<Vec<IbiRecord>, SymError> {
    let file = File::open(path).map_err(SymError::Io)?;
    let mut reader = BufReader::new(file);

    let mut out = Vec::new();

    loop {
        let mut ts_buf = [0u8; 4];
        let mut rr_buf = [0u8; 2];

        if reader.read(&mut ts_buf)? < 4 {
            break;
        }
        if reader.read(&mut rr_buf)? < 2 {
            break;
        }

        out.push(IbiRecord {
            timestamp: u32::from_le_bytes(ts_buf),
            rr_ms: u16::from_le_bytes(rr_buf),
        });
    }

    Ok(out)
}
