// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Application state and domain types for `symview`.
//!
//! Split by workflow domain; re-exported here so call sites keep `crate::app::*`.

mod biosym;
mod loaders;
mod loadsym;
mod spatial;
mod spatial_app;
mod state;
mod stats;
mod stats_app;
mod tab;
mod workflow;

pub use biosym::*;
pub use loadsym::*;
pub use spatial::*;
pub use state::App;
pub use stats::*;
pub use tab::*;
