// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

pub mod analysis;
pub mod generation;
pub mod peaks;
pub mod quality;

pub use analysis::{
    RespAnalysis, analyze_respiration, analyze_respiration_with, analyze_respiration_with_quality,
    detect_respiration_peaks, detect_respiration_peaks_with, resp_peak_finder, resp_signal,
    summarize_respiration,
};
pub use generation::{RespSimulationParams, RespTimeSeries, generate_respiration_timeseries};
pub use peaks::{RespPhasePeaks, phase_peak_indices};
pub use quality::{
    RespSignalQuality, resp_default_bandpass, resp_processing_for_quality,
    resp_processing_moderate, resp_processing_poor, resp_processing_reference,
};
