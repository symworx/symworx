// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! # symworx-embed
//!
//! Host-side streaming for embedded biosignal devices (Arduino-style PPG).
//!
//! Focus is the **host path**:
//! - JSON-line protocol for device PPG / vitals lines
//! - Subject-centric naming (`sid` on the wire)
//! - Ring buffers for live UI / analysis windows
//! - Simulator and optional serial sources
//!
//! Device firmware stays out of this crate.
//!
//! ## Features
//! - `simulate` (default) — synthetic vitals stream
//! - `serial` — serial port JSON-line reader
//!
//! ## Quick example
//!
//! ```
//! use symworx_embed::protocol::parse_json_line;
//!
//! let line = r#"{"red":1,"ir":2,"bpm":70.0,"bpm_avg":71,"ts":100}"#;
//! let sample = parse_json_line(line).unwrap().unwrap();
//! assert_eq!(sample.ir, Some(2));
//! ```

#![warn(missing_docs)]

pub mod buffer;
pub mod error;
pub mod protocol;
pub mod source;
pub mod status;
pub mod types;

#[cfg(feature = "simulate")]
pub mod simulate;

#[cfg(feature = "serial")]
pub mod serial;

pub use buffer::{
    Channel,
    SampleRing,
};
pub use error::{
    EmbedError,
    Result,
};
pub use protocol::{
    enrich,
    parse_json_line,
    sample_to_json_line,
};
#[cfg(feature = "serial")]
pub use serial::{
    SerialConfig,
    SerialSource,
};
#[cfg(feature = "simulate")]
pub use simulate::{
    SimulatorConfig,
    SimulatorSource,
};
pub use source::StreamSource;
pub use status::{
    VitalsStatus,
    VitalsThresholds,
    analyze_vitals,
    analyze_vitals_with,
};
pub use types::{
    SourceKind,
    StreamSample,
};

/// Crate version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
