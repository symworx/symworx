// Copyright (C) 2026 cSYMd, All rights reserved.

use symworx_error::SymError;

pub trait SymReader {
    type Output;

    fn read(path: &str) -> Result<Self::Output, SymError>;
}

pub trait SymWriter {
    type Input;

    fn write(path: &str, data: &Self::Input) -> Result<(), SymError>;
}
