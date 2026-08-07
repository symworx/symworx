// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Signal processing filters.
//!
//! This module contains various digital filters organized by category:
//! - Linear IIR/FIR filters
//! - Adaptive filters
//! - Nonlinear / state estimation filters:
//!   - `KalmanFilter` — linear-Gaussian + RTS
//!   - `ExtendedKalmanFilter` / `UnscentedKalmanFilter` — nonlinear
//!   - `KalmanFilter1D` — simple 1D constant-velocity tracker
//! - Time-frequency transforms

/// Adaptive algorithms
pub mod adaptive;

/// Linear algorithms
pub mod linear;

/// Nonlinear algorithms
pub mod nonlinear;

/// Time-frequency algorithms
pub mod time_frequency;

// Linear filters
// Adaptive filters
pub use adaptive::{
    LmsFilter,
    NlmsFilter,
    adaptive_mean_filter,
    adaptive_median_filter,
};
pub use linear::{
    BandpassFilter,
    ChebyshevFilter,
    // Add more as you implement them (FIR, Bessel, etc.)
};
// Nonlinear / estimation filters
pub use nonlinear::{
    ExtendedKalmanFilter,
    KalmanFilter,   // the primary general multivariate + RTS version
    KalmanFilter1D, // simple 1D constant-velocity tracker
    RlsFilter,
    UkfParams,
    UnscentedKalmanFilter,
    numerical_jacobian,
    sigma_points,
};
// Time-frequency analysis
pub use time_frequency::{
    AnalyticSignal,

    CwtResult,
    StftResult,
    WaveletType,
    WindowType,

    // Wavelet
    cwt,
    cwt_mexhat,
    cwt_morlet,
    // Hilbert
    hilbert,
    // STFT
    stft,
};
