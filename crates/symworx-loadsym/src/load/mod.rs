// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

/// Acute/Chronic work/load ratio
pub mod acwr;
/// Soft default planning goal from form / fatigue / ACLi
pub mod goal_suggest;
/// Mechanical load calculations
pub mod mechanical;
/// Exercise monotony and load
pub mod monotony;
/// Multi-day load planning (goal-conditioned optimization)
pub mod optimization;
/// Physiological load calculations
pub mod physiological;
/// Pulse-response / fitness–fatigue (Banister / PMC) model
pub mod pulse_response;

// re-exports of specific functions
pub use acwr::{
    AcwrSnapshot,
    RiskLevel,
    classify_acwr,
    compute_acute_chronic,
    compute_acwr_series,
    compute_ewma_acute_chronic,
};
pub use goal_suggest::{
    GoalSuggestParams,
    GoalSuggestion,
    suggest_load_goal,
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
#[allow(deprecated)]
pub use optimization::{
    LoadGoal,
    LoadPlan,
    MAX_HORIZON_DAYS,
    OptimizationThresholds,
    optimize_load,
    optimize_load_plan,
};
pub use physiological::calculate_physiological_load;
pub use pulse_response::{
    PulseResponseParams,
    PulseResponseSeries,
    PulseResponseState,
    PulseUpdateRule,
    estimate_recovery_days,
    forecast_pulse_response,
    forecast_with_constant_load,
    simulate_pulse_response,
    simulate_pulse_response_continuous,
    step_pulse_response,
    unit_impulse_response,
};
