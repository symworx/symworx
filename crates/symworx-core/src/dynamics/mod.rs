// core/dynamics/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

pub mod embedding;
pub mod entropy;
pub mod rqa;

pub use embedding::{edim, fnn};
pub use entropy::{sample_entropy};
