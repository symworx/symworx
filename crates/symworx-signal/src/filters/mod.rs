// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Signal processing filters.
//!
//! This module contains various digital filters organized by category:
//! - Linear IIR/FIR filters
//! - Adaptive filters
//! - Nonlinear / state estimation filters (primary `KalmanFilter` is the general
//!   multivariate state-space version with RTS smoothing; the simple 1D tracker
//!   is available as `KalmanFilter1D`)
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
    KalmanFilter,       // the primary general multivariate + RTS version
    KalmanFilter1D,     // the simple legacy 1D constant-velocity tracker
    RlsFilter,
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
