// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Shared physiology analysis primitives built on [`symworx_core`].

pub mod hrv;
pub mod intervals;
pub mod peaks;
pub mod processing;
pub mod signal;
pub mod stats;

pub use hrv::{
    compute_hrv_metrics,
    HrvMetrics
};
pub use intervals::IntervalSeries;
pub use peaks::{
    detect_intervals,
    detect_peaks,
    local_maxima_indices,
    peaks_to_intervals,
    PhysiologyPeak,
};
pub use processing::{
    apply_bandpass,
    apply_peak_overrides,
    preprocess_signal,
    BandpassParams,
    PeakDetectionParams,
    PhysiologyProcessingParams,
};
pub use signal::PhysiologySignal;
pub use stats::{
    summarize_signal,
    PhysiologySummary
};

// Re-export core peak types used across physiology.
pub use symworx_core::{
    Peak, 
    PeakDetect, 
    PeakFinderBuilder
};
