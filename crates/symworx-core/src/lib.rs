// core/src/lib.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(improper_ctypes_definitions)]

pub mod io;
pub mod math;
pub mod errors;
pub mod filters;
pub mod dynamics;
pub mod processing;
pub mod statistics;

pub use io::*;
pub use math::*;
pub use errors::*;
pub use filters::*;
pub use dynamics::*;
pub use processing::*;
pub use statistics::*;
