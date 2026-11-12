// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

use core::fmt;

/// Failure from an NPU entry point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpuError {
    /// Shape, radius, or theiler problem in the request.
    InvalidInput(&'static str),

    /// Packed mask length overflows.
    Overflow,
}

impl fmt::Display for NpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(msg) => f.write_str(msg),
            Self::Overflow => f.write_str("packed mask length overflows"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NpuError {}
