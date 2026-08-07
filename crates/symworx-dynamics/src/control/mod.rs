// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Linear systems and elementary feedback control.
//!
//! Discrete LTI plants, state feedback, and PID. Pair with
//! `symworx-signal::KalmanFilter` for state estimation (separate crate to
//! avoid dependency cycles).

mod lti;
mod pid;

pub use lti::{
    LtiDiscrete,
    LtiSimResult,
};
pub use pid::{
    Pid,
    PidConfig,
};
