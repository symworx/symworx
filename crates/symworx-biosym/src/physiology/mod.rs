// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Physiology module: PPG and respiration generation and analysis.

pub mod common;
pub mod ppg;
pub mod respiration;

pub use common::{
    BandpassParams,
    HrvMetrics,
    IntervalSeries,
    Peak,
    PeakDetect,
    PeakDetectionParams,
    PeakFinderBuilder,
    PhysiologyPeak,
    PhysiologyProcessingParams,
    PhysiologySignal,
    PhysiologySummary,
    apply_bandpass,
    apply_peak_overrides,
    compute_hrv_metrics,
    detect_intervals,
    detect_peaks,
    local_maxima_indices,
    peaks_to_intervals,
    preprocess_signal,
    summarize_signal,
};
pub use ppg::{
    PPGNoiseConfig,
    PPGSignalQuality,
    PPGSimulationParams,
    PPGTimeSeries,
    PpgAnalysis,
    analyze_ppg,
    analyze_ppg_with,
    analyze_ppg_with_quality,
    detect_ppg_peaks,
    detect_ppg_peaks_with,
    generate_ppg_timeseries,
    generate_ppg_waveform,
    ppg_default_bandpass,
    ppg_peak_finder,
    ppg_processing_for_quality,
    ppg_signal,
    summarize_ppg,
};
pub use respiration::{
    RespAnalysis,
    RespPhasePeaks,
    RespSignalQuality,
    RespSimulationParams,
    RespTimeSeries,
    analyze_respiration,
    analyze_respiration_with,
    analyze_respiration_with_quality,
    detect_respiration_peaks,
    detect_respiration_peaks_with,
    generate_respiration_timeseries,
    phase_peak_indices,
    resp_default_bandpass,
    resp_peak_finder,
    resp_processing_for_quality,
    resp_signal,
    summarize_respiration,
};
