#![allow(unused_imports)]
#![allow(dead_code)]

// core/src/io/edf.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use crate::errors::SymError;
use crate::io::traits::SymReader;
use ::edf::EdfFile;

pub struct EdfReader;

impl SymReader for EdfReader {
    type Output = Vec<Vec<f64>>;

    fn read(path: &str) -> Result<Self::Output, SymError> {
        let mut edf = EdfFile::open(path)?;
        let signals = edf.signals()?;

        let mut out = Vec::new();
        for s in signals {
            out.push(s.samples_f64()?);
        }

        Ok(out)
    }
}
