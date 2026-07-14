// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

/// Extended Kalman filter (nonlinear models + Jacobians).
pub mod ekf;
/// Primary general-purpose Kalman filter (multivariate state + observations,
/// control inputs, and RTS smoothing).
pub mod kalman;
/// Simple 1D constant-velocity Kalman filter (legacy / convenience name).
pub mod kalman1d;
/// Recursive least squares
pub mod rls;
/// Unscented Kalman filter (sigma-point nonlinear filtering).
pub mod ukf;

pub use ekf::{
    ExtendedKalmanFilter,
    numerical_jacobian,
};
pub use kalman::KalmanFilter;
pub use kalman1d::KalmanFilter1D;
pub use rls::*;
pub use ukf::{
    UkfParams,
    UnscentedKalmanFilter,
    sigma_points,
};
