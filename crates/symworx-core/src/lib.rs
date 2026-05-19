// crates/symworx-core/src/lib.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

#![warn(missing_docs)]

// ==========================================================
// Core re-exports 
// ==========================================================

pub use symworx_error as error;
pub use symworx_error::SymError;

// Main modules 
pub use symworx_math as math;
pub use symworx_io as io;
pub use symworx_signal as signal;
pub use symworx_stats as stats;
pub use symworx_dynamics as dynamics;
pub use symworx_backend as backend;

// Re-export of commonly used items
pub use symworx_signal::processing::{Peak, PeakDetect, PeakFinderBuilder};
pub use symworx_signal::filters;

pub use symworx_io::traits::*;
pub use symworx_stats::basic::*;

// ==========================================================
// Version info
// ==========================================================
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
