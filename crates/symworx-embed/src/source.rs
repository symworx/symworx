// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Stream source trait for serial, simulator, and future transports.

use crate::{
    error::Result,
    types::StreamSample,
};

/// Blocking iterator-style source of [`StreamSample`]s.
///
/// Returns `Ok(None)` on clean end-of-stream (rare for live sources).
/// Transient empty reads (e.g. serial timeout with no line) should return
/// `Ok(None)` only when the source is finished; live sources typically block
/// or return a sample. See each implementor.
pub trait StreamSource {
    /// Pull the next sample, or `None` if the stream has ended.
    fn next_sample(&mut self) -> Result<Option<StreamSample>>;
}
