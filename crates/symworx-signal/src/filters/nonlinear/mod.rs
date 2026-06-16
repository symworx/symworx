// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

/// Primary general-purpose Kalman filter (multivariate state + observations,
/// control inputs, and RTS smoothing).
pub mod kalman;
/// Simple 1D constant-velocity Kalman filter (legacy / convenience name).
pub mod kalman1d;
/// Recursive least squares
pub mod rls;

pub use kalman::KalmanFilter;
pub use kalman1d::KalmanFilter1D;
pub use rls::*;
