// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

use symworx_error::SymError;

/// SymReader trait for reading structured data.
pub trait SymReader {
    /// Type returned by [`SymReader::read`]
    type Output;
    /// Reads the data at given path.
    fn read(path: &str) -> Result<Self::Output, SymError>;
}

/// SymWriter trait for writing structured data.
pub trait SymWriter {
    /// Type accepted by [`SymReader::write`]
    type Input;
    /// Write the data to a given path.
    fn write(path: &str, data: &Self::Input) -> Result<(), SymError>;
}
