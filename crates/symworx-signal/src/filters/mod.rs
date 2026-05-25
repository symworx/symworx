// Copyright (C) 2026 cSYMd, All rights reserved.

//! Signal processing filters.
//!
//! This module contains various digital filters organized by category:
//! - Linear IIR/FIR filters
//! - Adaptive filters
//! - Nonlinear / state estimation filters
//! - Time-frequency transforms

pub mod adaptive;
pub mod linear;
pub mod nonlinear;
pub mod time_frequency;


// Linear filters
pub use linear::{
    BandpassFilter,
    ChebyshevFilter,
    // Add more as you implement them (FIR, Bessel, etc.)
};

// Adaptive filters
pub use adaptive::{
    adaptive_mean_filter,
    adaptive_median_filter,
    LmsFilter,
    NlmsFilter,
};

// Nonlinear / estimation filters
pub use nonlinear::{
    KalmanFilter,
    RlsFilter,
};

// Time-frequency analysis
pub use time_frequency::{
    // Hilbert
    hilbert,
    AnalyticSignal,

    // STFT
    stft,
    StftResult,
    WindowType,

    // Wavelet
    cwt,
    cwt_morlet,
    cwt_mexhat,
    CwtResult,
    WaveletType,
};
