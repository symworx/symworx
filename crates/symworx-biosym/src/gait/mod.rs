// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Gait modeling and analysis.
//!
//! Core types for gait simulation parameters (`GaitParams`) and derived
//! spatiotemporal metrics (`GaitData`, `GaitStats`).
//!
//! Provides stride/step timing, cadence, length calculations, vertical oscillation,
//! symmetry, and full `GaitAnalysis` (with quality-aware stride detection from signals).
//! Designed for use with RQA and nonlinear dynamics tooling in `symworx-dynamics`.
//!
//! See `detect_gait_strides*`, `analyze_gait*`, `GaitAnalysis`, `GaitData`, `GaitStats`,
//! and `metrics` for helpers. Quality presets and processing mirror the physiology module.
//!
//! ## Example
//! ```ignore
//! use symworx_biosym::GaitParams;
//! use symworx_biosym::gait::{GaitData, GaitStats};
//! // or via grouping:
//! use symworx_biosym::biomechanics::gait::{GaitData, GaitStats};
//!
//! let params = GaitParams::default().with_defaults();
//! let mut data = GaitData::new(100.0);
//! data.stride_times = Some(ndarray::array![0.0, 1.0, 2.0]);
//! let _ = data.calculate_stride_intervals();
//! let stats: GaitStats = data.to_gait_stats(Some(1.3));
//!
//! // Or detect from a signal + analyze
//! let signal = vec![0.0, 0.0, 1.0, 0.0, 1.0, 0.0]; // toy
//! let analysis = analyze_gait_signal(&signal, 10.0);
//! ```

mod analysis;
mod data;
mod metrics;
mod params;
mod processing;
mod quality;

pub use analysis::{
    GaitAnalysis,
    analyze_gait,
    analyze_gait_from_times,
    analyze_gait_signal,
    analyze_gait_signal_with_quality,
    detect_gait_strides,
    detect_gait_strides_with,
    detect_gait_strides_with_quality,
    gait_peak_finder,
};
pub use data::GaitData;
pub use metrics::{
    GaitStats,
    compute_gait_stats,
};
pub use params::GaitParams;
pub use processing::{
    gait_default_bandpass,
    gait_processing_for_quality,
    gait_processing_high,
    gait_processing_moderate,
    gait_processing_poor,
    gait_processing_reference,
};
pub use quality::GaitSignalQuality;

// Re-export IntervalSeries for gait event analysis (time-based stride events etc.).
// Preferred location is `common::IntervalSeries` (also available at crate root).
pub use crate::common::IntervalSeries;
