// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! # symworx-dynamics
//!
//! Nonlinear dynamics, chaos, and recurrence analysis tools for SymWorx.
//!
//! This crate provides utilities for analyzing complex physiological and
//! biomechanical time series using techniques from nonlinear dynamical systems.
//!
//! ## Modules
//!
//! - [`embedding`] — Phase space reconstruction (`edim`, false nearest neighbors)
//! - [`entropy`] — Sample entropy and multiscale entropy
//! - [`rqa`] — Recurrence Quantification Analysis (RQA) and cross-recurrence (cRQA)
//! - [`phase`] — Relative phase, Kuramoto order, cluster-phase (pre-extracted phases)
//! - [`dmd`] — Dynamic Mode Decomposition (data-driven linear operators)
//! - [`koopman`] — Extended DMD / finite-dimensional Koopman operators
//! - [`sindy`] — Sparse identification of nonlinear dynamics (STLS)
//! - [`sindyc`] — SINDYc (SINDy with control inputs)
//! - [`control`] — Discrete LTI plants, state feedback, PID
//!
//! These tools are especially useful for gait, heart rate, respiration, and
//! other quasi-periodic biological signals.

#![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/symworx-dynamics")]

// Modules
/// Linear systems and elementary feedback control.
pub mod control;
/// Dynamic Mode Decomposition (exact / SVD-based).
pub mod dmd;
/// Embedding dimension calculations (e.g., edim, fnn).
pub mod embedding;
/// Entropy measures (e.g., sample entropy, multiscale entropy).
pub mod entropy;
/// Extended DMD / Koopman operator approximation.
pub mod koopman;
/// Relative phase, Kuramoto order, and cluster-phase synchrony.
pub mod phase;
/// Recurrence and cross-recurrence calculations.
pub mod rqa;
/// Sparse Identification of Nonlinear Dynamics (SINDy).
pub mod sindy;
/// SINDy with control (SINDYc).
pub mod sindyc;

// Re-exports
pub use control::{
    LtiDiscrete,
    LtiSimResult,
    Pid,
    PidConfig,
};
pub use dmd::{
    DmdConfig,
    DmdResult,
    dmd,
    dmd_pair,
    snapshots_from_embedding,
    snapshots_from_states,
};
pub use embedding::{
    edim,
    fnn,
};
pub use entropy::{
    multiscale_entropy,
    sample_entropy,
};
pub use koopman::{
    Dictionary,
    EdmdConfig,
    EdmdResult,
    decode_state,
    edmd,
    edmd_pair,
    lift_snapshots,
    lift_state,
};
pub use phase::{
    ClusterPhaseResult,
    PhaseError,
    PhaseRelation,
    RelativePhaseSummary,
    cluster_phase,
    kuramoto_order,
    kuramoto_order_series,
    relative_phase,
    summarize_relative_phase,
};
pub use rqa::{
    CrossRecurrencePlot,
    DEFAULT_LMIN,
    DEFAULT_VMIN,
    RecurrencePlot,
    RqaResult,
    crqa,
    rqa,
    rqa_from_trajectory,
};
pub use sindy::{
    SindyConfig,
    SindyResult,
    library_matrix_rows,
    sindy,
    sindy_with_derivatives,
};
pub use sindyc::{
    SindycConfig,
    SindycResult,
    library_dim_xu,
    library_matrix_xu,
    lift_xu,
    sindyc,
    sindyc_with_derivatives,
};

// Version info
/// Current version of the `symworx-dynamics` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
