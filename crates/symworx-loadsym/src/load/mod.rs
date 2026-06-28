// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

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
pub use mechanical::{
    MovementLoadMetrics,
    calculate_mechanical_load,
    compute_movement_load_metrics,
    estimate_external_load_from_normalized_pace,
    estimate_external_load_from_pace,
};
pub use monotony::{
    compute_monotony,
    compute_strain,
};
pub use optimization::optimize_load;
pub use physiological::calculate_physiological_load;
