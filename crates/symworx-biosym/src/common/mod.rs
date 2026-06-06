// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Common shared primitives across SymWorx BioSym domains
//! (physiology, biomechanics, biosystems, etc.).
//!
//! This is the preferred home for cross-cutting utilities such as
//! `IntervalSeries` (for RR, breath, stride, and other event intervals) and
//! processing parameters (`common::processing`) so that future domains
//! can rely on them.

pub mod hrv;
pub mod intervals;
pub mod peaks;
pub mod processing;
pub mod signal;
pub mod stats;

pub use hrv::{
    HrvMetrics,
    compute_hrv_metrics,
};
pub use intervals::IntervalSeries;
pub use peaks::{
    PhysiologyPeak,
    detect_intervals,
    detect_peaks,
    local_maxima_indices,
    peaks_to_intervals,
};
pub use processing::{
    BandpassParams,
    PeakDetectionParams,
    PhysiologyProcessingParams,
    apply_bandpass,
    apply_peak_overrides,
    preprocess_signal,
};
pub use signal::PhysiologySignal;
pub use stats::{
    PhysiologySummary,
    summarize_signal,
};
// Re-export core peak types used across physiology.
pub use symworx_core::{
    Peak,
    PeakDetect,
    PeakFinderBuilder,
};
