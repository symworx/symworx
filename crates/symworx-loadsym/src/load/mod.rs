// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

pub mod acwr;
pub mod mechanical;
pub mod monotony;
pub mod optimization;
pub mod physiological;

// re-exports of specific functions
pub use acwr::{
    classify_acwr, compute_acute_chronic, compute_acwr_series, compute_ewma_acute_chronic,
    AcwrSnapshot, RiskLevel,
};
pub use mechanical::calculate_mechanical_load;
pub use monotony::{compute_monotony, compute_strain};
pub use optimization::optimize_load;
pub use physiological::calculate_physiological_load;
