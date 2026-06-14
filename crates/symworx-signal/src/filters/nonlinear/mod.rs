// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

/// Simple 1D constant-velocity Kalman filter (legacy / convenience name).
pub mod kalman1d;
/// Primary general-purpose Kalman filter (multivariate state + observations,
/// control inputs, and RTS smoothing).
pub mod kalman;
/// Recursive least squares
pub mod rls;

pub use kalman1d::KalmanFilter1D;
pub use kalman::KalmanFilter;
pub use rls::*;
