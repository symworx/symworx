// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

use symworx_error::SymError;

pub trait SymReader {
    type Output;

    fn read(path: &str) -> Result<Self::Output, SymError>;
}

pub trait SymWriter {
    type Input;

    fn write(path: &str, data: &Self::Input) -> Result<(), SymError>;
}
