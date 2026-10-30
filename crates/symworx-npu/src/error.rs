// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

/// Failure from an NPU entry point.
#[derive(Debug, thiserror::Error)]
pub enum NpuError {
    /// Shape, radius, or theiler problem in the request.
    #[error("{0}")]
    InvalidInput &'static str,

    /// Packed mask length overflows.
    #[error("packed mask length overflows")]
    Overflow,
}
