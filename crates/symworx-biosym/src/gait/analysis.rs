// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

use ndarray::Array1;
use symworx_core::PeakFinderBuilder;

use super::{GaitData, GaitSignalQuality, GaitStats, processing::gait_processing_for_quality};
use crate::{
    common::IntervalSeries,
    processing::{PhysiologyProcessingParams, apply_bandpass},
};

const GAIT_BASE_MIN_INTERVAL_SEC: f64 = 0.5;
const GAIT_BASE_PROMINENCE: f64 = 0.08;
const GAIT_BASE_HEIGHT: f64 = 0.0;

/// Results of gait stride/event analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct GaitAnalysis {
    pub stats: GaitStats,
    pub intervals: IntervalSeries,
}

/// Gait peak finder builder with optional processing overrides (~2 Hz upper for strides).
pub fn gait_peak_finder<'a>(
    signal: &'a [f64],
    fs: f64,
    processing: &PhysiologyProcessingParams,
) -> PeakFinderBuilder<'a> {
    crate::common::processing::apply_peak_overrides(
        PeakFinderBuilder::from_slice(signal),
        fs,
        GAIT_BASE_MIN_INTERVAL_SEC,
        GAIT_BASE_PROMINENCE,
        GAIT_BASE_HEIGHT,
        &processing.peaks,
    )
}

/// Detect stride events from a reference signal (e.g. pelvis vertical position or hip angle).
/// No bandpass (backward compatible default).
pub fn detect_gait_strides(signal: &[f64], fs: f64) -> IntervalSeries {
    detect_gait_strides_with(signal, fs, &PhysiologyProcessingParams::none())
        .expect("default detection has no bandpass")
}

/// Detect with explicit processing (bandpass + peak overrides).
pub fn detect_gait_strides_with(
    signal: &[f64],
    fs: f64,
    processing: &PhysiologyProcessingParams,
) -> Result<IntervalSeries, &'static str> {
    let processed = if let Some(ref bp) = processing.bandpass {
        apply_bandpass(signal, fs, bp)?
    } else {
        signal.to_vec()
    };
    let finder = gait_peak_finder(&processed, fs, processing);
    let peak_list = finder.find();
    let indices: Vec<usize> = peak_list.into_iter().map(|p| p.index).collect();
    Ok(IntervalSeries::from_peak_indices(&indices, fs))
}

/// Detect using a quality preset.
pub fn detect_gait_strides_with_quality(
    signal: &[f64],
    fs: f64,
    quality: GaitSignalQuality,
) -> IntervalSeries {
    detect_gait_strides_with(signal, fs, &gait_processing_for_quality(quality))
        .expect("quality presets use valid bandpass cutoffs")
}

/// Analyze from pre-detected stride times (seconds). Computes stats + wraps as IntervalSeries.
pub fn analyze_gait_from_times(stride_times: &[f64], walking_speed: Option<f64>) -> GaitAnalysis {
    let intervals = if stride_times.len() < 2 {
        IntervalSeries::default()
    } else {
        IntervalSeries::from_times(stride_times)
    };

    // Reuse GaitData calculators for lengths, cadence, etc. (fs is only for index-based osc etc.)
    let mut data = GaitData::new(100.0);
    data.stride_times = Some(Array1::from(stride_times.to_vec()));
    data.calculate_stride_intervals();
    let speed = walking_speed.unwrap_or(1.3);
    data.calculate_stride_length(Some(speed));
    data.calculate_step_length();

    let stats = data.to_gait_stats(Some(speed));
    GaitAnalysis { stats, intervals }
}

/// Full analysis from a raw reference signal (detects strides then stats).
pub fn analyze_gait_signal(signal: &[f64], fs: f64) -> GaitAnalysis {
    let intervals = detect_gait_strides(signal, fs);
    analyze_gait_from_times(&intervals.peak_times, None)
}

/// Analysis using quality preset on a raw signal.
pub fn analyze_gait_signal_with_quality(
    signal: &[f64],
    fs: f64,
    quality: GaitSignalQuality,
) -> GaitAnalysis {
    let intervals = detect_gait_strides_with_quality(signal, fs, quality);
    analyze_gait_from_times(&intervals.peak_times, None)
}

/// Convenience: populate stats on an existing GaitData (e.g. after setting stride_times or detecting).
/// Returns a snapshot GaitAnalysis.
pub fn analyze_gait(data: &mut GaitData, walking_speed: Option<f64>) -> GaitAnalysis {
    if data.stride_intervals.is_none() && data.stride_times.is_some() {
        data.calculate_stride_intervals();
    }
    let speed = walking_speed.unwrap_or(1.3);
    if data.stride_length.is_none() {
        data.calculate_stride_length(Some(speed));
    }
    if data.step_length.is_none() {
        data.calculate_step_length();
    }
    let stats = data.to_gait_stats(Some(speed));
    let times_for_intervals = data
        .stride_times
        .as_ref()
        .map(|t| t.to_vec())
        .unwrap_or_default();
    let intervals = if times_for_intervals.len() >= 2 {
        IntervalSeries::from_times(&times_for_intervals)
    } else {
        IntervalSeries::default()
    };
    GaitAnalysis { stats, intervals }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_stride_signal(fs: f64, stride_sec: f64, n_strides: usize) -> (Vec<f64>, Vec<f64>) {
        let dur = stride_sec * n_strides as f64;
        let n = (dur * fs).round() as usize;
        let mut t = Vec::with_capacity(n);
        let mut sig = Vec::with_capacity(n);
        for i in 0..n {
            let tt = i as f64 / fs;
            t.push(tt);
            // Pulse with a distinct apex (1.0) in the center of each "on" window so that
            // the strict local-max finder detects it even without bandpass. The overall
            // pulse shape is close to the original so that bandpass + min_height=0.05
            // in quality presets still yields usable peaks for the tests.
            let phase = (tt % stride_sec) / stride_sec;
            let in_pulse = phase < 0.1;
            let mut val = if in_pulse { 0.5 } else { 0.0 };
            if in_pulse {
                let pulse_phase = phase / 0.1;
                if (pulse_phase - 0.5).abs() < 0.02 {
                    val = 1.0;
                }
            }
            sig.push(val);
        }
        (t, sig)
    }

    #[test]
    fn detect_gait_strides_on_synthetic() {
        let fs = 100.0;
        let stride = 1.2;
        let (_, sig) = synthetic_stride_signal(fs, stride, 10);
        let ints = detect_gait_strides(&sig, fs);
        assert!(!ints.peak_indices.is_empty());
        // Should recover approx the stride rate
        let mean_rate = ints.mean_instantaneous_rate();
        assert!(mean_rate > 40.0 && mean_rate < 60.0); // ~50 spm for 1.2s stride
    }

    #[test]
    fn analyze_from_times_matches_stats() {
        let times = vec![0.0, 1.15, 2.30, 3.45];
        let analysis = analyze_gait_from_times(&times, Some(1.3));
        assert!((analysis.stats.mean_stride_time_s - 1.15).abs() < 0.01);
        assert!(analysis.stats.cadence_steps_min.unwrap() > 100.0);
        assert!(!analysis.intervals.peak_times.is_empty());
    }

    #[test]
    fn analyze_signal_with_quality() {
        let fs = 100.0;
        let stride = 1.0;
        let (_, sig) = synthetic_stride_signal(fs, stride, 12);
        let a = analyze_gait_signal_with_quality(&sig, fs, GaitSignalQuality::Reference);
        assert!(a.stats.n_strides >= 4);
        assert!(a.stats.mean_stride_time_s > 0.5 && a.stats.mean_stride_time_s < 1.5);
    }

    #[test]
    fn gait_peak_finder_applies_overrides() {
        let sig = vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0];
        let fs = 10.0;
        let proc = PhysiologyProcessingParams {
            bandpass: None,
            peaks: crate::common::processing::PeakDetectionParams {
                min_interval_sec: Some(0.3),
                ..Default::default()
            },
        };
        let finder = gait_peak_finder(&sig, fs, &proc);
        let peaks = finder.find();
        assert!(!peaks.is_empty());
    }
}
