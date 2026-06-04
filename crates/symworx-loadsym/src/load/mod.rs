// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

/// Acute/Chronic work/load ratio
pub mod acwr;
/// Mechanical load calculations
pub mod mechanical;
/// Exercise monotony and load
pub mod monotony;
/// Load optimization algorithms
pub mod optimization;
/// Physiological load calculations
pub mod physiological;

// re-exports of specific functions
pub use acwr::{
    AcwrSnapshot,
    RiskLevel,
    classify_acwr,
    compute_acute_chronic,
    compute_acwr_series,
    compute_ewma_acute_chronic,
};
pub use mechanical::calculate_mechanical_load;
pub use monotony::{
    compute_monotony,
    compute_strain,
};
pub use optimization::optimize_load;
pub use physiological::calculate_physiological_load;
