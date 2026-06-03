// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

pub mod analysis;
pub mod generation;
pub mod noise;
pub mod processing;
pub mod quality;

pub use analysis::{
    analyze_ppg, analyze_ppg_with, analyze_ppg_with_quality, detect_ppg_peaks,
    detect_ppg_peaks_with, summarize_ppg, PpgAnalysis, ppg_peak_finder, ppg_signal,
};
pub use processing::{
    ppg_default_bandpass, ppg_processing_for_quality, ppg_processing_high,
    ppg_processing_moderate, ppg_processing_poor, ppg_processing_reference,
};
pub use generation::{
    PPGSimulationParams, PPGTimeSeries, generate_ppg_timeseries, generate_ppg_waveform,
};
pub use noise::PPGNoiseConfig;
pub use quality::PPGSignalQuality;
