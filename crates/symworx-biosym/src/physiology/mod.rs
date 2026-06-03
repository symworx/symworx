// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Physiology module: PPG and respiration generation and analysis.

pub mod common;
pub mod ppg;
pub mod respiration;

pub use common::{
    apply_bandpass, apply_peak_overrides, compute_hrv_metrics, detect_intervals, detect_peaks,
    local_maxima_indices, peaks_to_intervals, preprocess_signal, summarize_signal,
    BandpassParams, HrvMetrics, IntervalSeries, PeakDetectionParams, PhysiologyPeak,
    PhysiologyProcessingParams, PhysiologySignal, PhysiologySummary, Peak, PeakDetect,
    PeakFinderBuilder,
};
pub use ppg::{
    analyze_ppg, analyze_ppg_with, analyze_ppg_with_quality, detect_ppg_peaks,
    detect_ppg_peaks_with, ppg_default_bandpass, ppg_processing_for_quality, summarize_ppg,
    PPGNoiseConfig, PPGSignalQuality, PpgAnalysis, PPGSimulationParams, PPGTimeSeries,
    generate_ppg_timeseries, generate_ppg_waveform, ppg_peak_finder, ppg_signal,
};
pub use respiration::{
    analyze_respiration, analyze_respiration_with, analyze_respiration_with_quality,
    detect_respiration_peaks, detect_respiration_peaks_with, phase_peak_indices,
    resp_default_bandpass, resp_processing_for_quality, summarize_respiration, RespAnalysis,
    RespPhasePeaks, RespSignalQuality, RespSimulationParams, RespTimeSeries,
    generate_respiration_timeseries, resp_peak_finder, resp_signal,
};
