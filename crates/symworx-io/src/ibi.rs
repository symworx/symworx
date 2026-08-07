// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

use std::{
    fs::File,
    io::{
        BufReader,
        Read,
    },
};

use symworx_error::SymError;

/// IBI data record structure.
#[derive(Debug, Clone)]
pub struct IbiRecord {
    /// Timestamp corresponding to RR intervals.
    pub timestamp: u32,
    /// The RR interval (ms).
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
