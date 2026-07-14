// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Linear systems and elementary feedback control.
//!
//! Educational / research foundations aligned with Kim-style LTI analysis and
//! Brunton-style control of dynamical systems. Pair with
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
