// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! No-std recurrence masks and RQA statistics for embedded targets.

extern crate alloc;

mod error;
mod recurrence;
mod rqa;

pub use error::NpuError;
pub use recurrence::{
    RecurrenceMask,
    Trajectory,
    recurrence_mask,
};
pub use rqa::{
    RqaParams,
    RqaStats,
    rqa,
};
