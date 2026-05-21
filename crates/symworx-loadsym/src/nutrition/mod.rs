// symworx/crates/symworx-loadsym/src/nutrition/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

pub mod bodycomp;

pub use bodycomp::{
    calculate_bmr,
    calculate_tdee,
    ActivityLevel
};
