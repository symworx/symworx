// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! # symworx-spatialsym
//!
//! Spatial trajectory analysis and post-hoc interpretation of movement decisions.
//!
//! This crate provides **sport-agnostic** tools for working with 2D position time-series
//! of multiple agents (entities) and optional focal objects. The focus is on deriving
//! kinematics and using bidirectional (past + future) context to classify space-related
//! actions such as expansion, penetration, denial, and pressure.
//!
//! ## Key Concepts (idiomatic, reusable)
//! - [`geometry`] — `Point2`, `Vec2`, bearing/angle, basic ops (Copy-friendly).
//! - [`trajectory`] — Time-stamped position sequences (single + batched).
//! - [`kinematics`] — Velocity, speed, heading derivation using `symworx-math` series primitives.
//! - [`metrics`] — Pairwise distances and single-agent path linearity (vs the start→end chord).
//! - [`phase`] — Pairwise in-phase / out-of-phase effort and directional scoring.
//! - Space geometry primitives and decision classification are co-evolving (see high-priority work).
//! - [`space`] — Play-area bounds plus sport-agnostic markings; [`soccer`] has IFAB Law 1 presets.
//! - [`decision`] — `SpaceAction` enum and classifiers using historical + future windows.
//!
//! ## Design Notes
//! - All linear dimensions are in **meters** (per SymWorx convention).
//! - Prefer explicit `dt` or full `times` over hardcoded frame rates.
//! - Post-hoc analyses intentionally look both backward and forward in time.
//!
//! ## Terminology
//! Public types avoid sport-specific language:
//! - "Agent" / "Entity" (not player)
//! - "FocalObject" (not ball)
//! - "Arena" / "PlayArea" (not pitch)
//! - `SpaceAction::{Expansion, Penetration, Denial, Pressure}` (see `decision`).

#![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/symworx-spatialsym")]

// Modules (to be populated incrementally)
pub mod decision;
pub mod error;
pub mod geometry;
pub mod kinematics;
pub mod load;
pub mod metrics;
pub mod phase;
pub mod space;
pub mod synthetic;
pub mod trajectory;

// Re-exports for convenience (mirrors symworx-core / biosym style)
pub use decision::{
    AgentDecision,
    ClassifySpaceParams,
    DecisionFeatures,
    SpaceAction,
    classify_single_trajectory,
    classify_single_trajectory_with_params,
    classify_space_actions,
};
pub use error::{
    Result,
    SpatialError,
};
pub use geometry::{
    Point2,
    Vec2,
    bearing_between,
};
pub use kinematics::{
    EffortEvent,
    accel_decel_events,
    bearing_to_cardinal,
    count_accelerations_decelerations,
    derive_along_track_accels,
    derive_closing_accels,
    derive_headings,
    derive_scalar_accels,
    derive_speeds,
    derive_velocities,
    derive_velocities_from_times,
    future_bearings,
    heading_to_bearing,
    normalize_to_peak_pace,
    past_bearings,
};
pub use load::load_trajectories_csv;
#[cfg(feature = "async")]
pub use load::load_trajectories_csv_async;
pub use metrics::{
    PathLinearity,
    distances_to_focal,
    pairwise_distances,
    path_length,
    path_linearity,
    path_linearity_windows,
};
pub use phase::{
    DirectionalRelation,
    PairwiseClosing,
    PairwiseDirectionalPhase,
    PairwiseEffortPhase,
    PhaseWindow,
    accel_index_for_frame,
    pairwise_closing,
    pairwise_closing_at,
    pairwise_closing_series,
    pairwise_directional_phase,
    pairwise_directional_phase_at,
    pairwise_directional_phase_series,
    pairwise_effort_phase,
    pairwise_effort_phase_at,
    pairwise_effort_phase_series,
};
pub use space::{
    CenterCircle,
    EndBox,
    GoalSpec,
    PenaltyMark,
    PlayAreaMarkings,
    PlayingDimensions,
    soccer,
};
pub use synthetic::{
    SpatialEvent,
    build_agent_trajectories,
    generate_3v3_attack,
    generate_curved_trajectory,
    generate_event_driven,
    generate_ground_truth,
    generate_linear_trajectory,
    generate_noisy_trajectory,
};
pub use trajectory::{
    AgentTrajectories,
    GroupSummary,
    PlayerSummary,
    SpatialContext,
    SpatialFrame,
    Trajectory,
};

// Version info
/// Current version of the `symworx-spatialsym` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
