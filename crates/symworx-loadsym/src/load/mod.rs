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
    RideMetrics,
    calculate_mechanical_load,
    compute_movement_load_metrics,
    compute_ride_metrics,
    compute_ride_metrics_from_activity,
    estimate_external_load_from_normalized_pace,
    estimate_external_load_from_pace,
    // Power / intensity primitives for workout analysis
    exceedance_marker_string,
    find_exceedance_regions,
    // Synthetic demo generator (explicit use only; not loaded by default)
    generate_demo_daily_loads,
    highest_rolling,
    peak,
    ride_load_from_metrics,
};
pub use monotony::{
    compute_monotony,
    compute_strain,
};
pub use optimization::optimize_load;
pub use physiological::calculate_physiological_load;
