// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

use symworx_core::PeakFinderBuilder;

use super::{
    RespTimeSeries,
    peaks::{
        RespPhasePeaks,
        phase_peak_indices,
        phase_peak_intervals_sec,
    },
    quality::{
        RespSignalQuality,
        resp_processing_for_quality,
    },
};
use crate::common::{
    IntervalSeries,
    PhysiologyProcessingParams,
    PhysiologySignal,
    PhysiologySummary,
    apply_peak_overrides,
    detect_intervals,
    preprocess_signal,
    summarize_signal,
};

const RESP_BASE_MIN_INTERVAL_SEC: f64 = 2.0;
const RESP_BASE_PROMINENCE: f64 = 0.05;
const RESP_BASE_HEIGHT: f64 = 0.05;

/// Results of respiration waveform analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct RespAnalysis {
    pub summary: PhysiologySummary,
    pub intervals: IntervalSeries,
    pub mean_brpm: f64,
    /// Even-indexed inter-peak intervals (legacy insp/exp split on detected peaks).
    pub insp_intervals_sec: Vec<f64>,
    pub exp_intervals_sec: Vec<f64>,
    pub phase_peaks: RespPhasePeaks,
    /// Intervals between consecutive inhalation peaks.
    pub insp_peak_intervals_sec: Vec<f64>,
    /// Intervals between consecutive exhalation peaks.
    pub exp_peak_intervals_sec: Vec<f64>,
}

/// Build a [`PhysiologySignal`] from a respiration time series (flow channel).
pub fn resp_signal(ts: &RespTimeSeries) -> PhysiologySignal {
    PhysiologySignal::with_times(50.0, ts.flow.clone(), ts.times.clone())
}

/// Respiration peak finder with optional processing overrides (~30 brpm upper bound).
pub fn resp_peak_finder<'a>(
    signal: &'a PhysiologySignal,
    processing: &PhysiologyProcessingParams,
) -> PeakFinderBuilder<'a> {
    apply_peak_overrides(
        PeakFinderBuilder::from_slice(&signal.samples),
        signal.fs,
        RESP_BASE_MIN_INTERVAL_SEC,
        RESP_BASE_PROMINENCE,
        RESP_BASE_HEIGHT,
        &processing.peaks,
    )
}

/// Detect breathing-cycle peaks (no bandpass).
pub fn detect_respiration_peaks(ts: &RespTimeSeries) -> IntervalSeries {
    detect_respiration_peaks_with(ts, &PhysiologyProcessingParams::none())
        .expect("default detection has no bandpass")
}

/// Detect peaks with optional bandpass and peak overrides.
pub fn detect_respiration_peaks_with(
    ts: &RespTimeSeries,
    processing: &PhysiologyProcessingParams,
) -> Result<IntervalSeries, &'static str> {
    let signal = preprocess_signal(resp_signal(ts), processing)?;
    Ok(detect_intervals(
        &signal,
        resp_peak_finder(&signal, processing),
    ))
}

/// Summarize a respiration recording (raw flow).
pub fn summarize_respiration(ts: &RespTimeSeries) -> PhysiologySummary {
    summarize_signal(&resp_signal(ts))
}

fn build_resp_analysis(signal: &PhysiologySignal, intervals: IntervalSeries) -> RespAnalysis {
    let summary = summarize_signal(signal);

    let mean_brpm = if intervals.intervals_sec.is_empty() {
        intervals.mean_rate_over_window(summary.duration_sec)
    } else {
        intervals.mean_instantaneous_rate()
    };

    let (insp_intervals_sec, exp_intervals_sec) = intervals.alternating_phase_intervals();
    let phase_peaks = phase_peak_indices(&signal.samples);
    let insp_peak_intervals_sec =
        phase_peak_intervals_sec(&phase_peaks.inhalation_peak_indices, signal.fs);
    let exp_peak_intervals_sec =
        phase_peak_intervals_sec(&phase_peaks.exhalation_peak_indices, signal.fs);

    RespAnalysis {
        summary,
        intervals,
        mean_brpm,
        insp_intervals_sec,
        exp_intervals_sec,
        phase_peaks,
        insp_peak_intervals_sec,
        exp_peak_intervals_sec,
    }
}

/// Full analysis without bandpass (backward compatible).
pub fn analyze_respiration(ts: &RespTimeSeries) -> RespAnalysis {
    analyze_respiration_with(ts, &PhysiologyProcessingParams::none())
        .expect("default analysis has no bandpass")
}

/// Full analysis with explicit processing parameters.
pub fn analyze_respiration_with(
    ts: &RespTimeSeries,
    processing: &PhysiologyProcessingParams,
) -> Result<RespAnalysis, &'static str> {
    let signal = preprocess_signal(resp_signal(ts), processing)?;
    let intervals = detect_intervals(&signal, resp_peak_finder(&signal, processing));
    Ok(build_resp_analysis(&signal, intervals))
}

/// Analysis using quality-preset bandpass and peak settings.
pub fn analyze_respiration_with_quality(
    ts: &RespTimeSeries,
    quality: RespSignalQuality,
) -> RespAnalysis {
    analyze_respiration_with(ts, &resp_processing_for_quality(quality))
        .expect("quality presets use valid bandpass cutoffs")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        RespSimulationParams,
        generate_respiration_timeseries,
    };

    fn synthetic_resp_flow(fs: f64, brpm: f64, duration_sec: f64) -> RespTimeSeries {
        let n = (duration_sec * fs).round() as usize;
        let cycle_sec = 60.0 / brpm;
        let mut times = Vec::with_capacity(n);
        let mut flow = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f64 / fs;
            times.push(t);
            let phase = (t % cycle_sec) / cycle_sec;
            flow.push((2.0 * std::f64::consts::PI * phase).sin().max(0.0));
        }
        RespTimeSeries {
            times,
            flow,
            volume: vec![0.0; n],
            inhalation_peaks: vec![],
            exhalation_peaks: vec![],
        }
    }

    #[test]
    fn summarize_respiration_basic() {
        let ts = synthetic_resp_flow(50.0, 12.0, 60.0);
        let summary = summarize_respiration(&ts);
        assert!((summary.duration_sec - 60.0).abs() < 0.1);
        assert!(summary.mean.is_finite());
    }

    #[test]
    fn detect_respiration_peaks_on_generated() {
        let params = RespSimulationParams {
            brpm: 12.0,
            dur_min: 0.5,
            fs: 50.0,
            noise_level: 0.0,
            ..Default::default()
        };
        let ts = generate_respiration_timeseries(&params);
        let intervals = detect_respiration_peaks(&ts);
        assert!(
            !intervals.peak_indices.is_empty(),
            "expected peaks on generated respiration flow"
        );
    }

    #[test]
    fn analyze_respiration_generated() {
        let params = RespSimulationParams {
            brpm: 12.0,
            dur_min: 1.0,
            fs: 50.0,
            noise_level: 0.0,
            ..Default::default()
        };
        let ts = generate_respiration_timeseries(&params);
        let analysis = analyze_respiration(&ts);
        assert!(
            (analysis.mean_brpm - 12.0).abs() < 4.0,
            "mean BRPM {:.1} should be near 12",
            analysis.mean_brpm
        );
    }

    #[test]
    fn analyze_respiration_with_bandpass() {
        let params = RespSimulationParams {
            brpm: 12.0,
            dur_min: 1.0,
            fs: 50.0,
            noise_level: 0.05,
            ..Default::default()
        };
        let ts = generate_respiration_timeseries(&params);
        let analysis = analyze_respiration_with_quality(&ts, RespSignalQuality::Reference);
        assert!(
            !analysis.intervals.peak_indices.is_empty(),
            "bandpass analysis should detect breaths"
        );
        assert!(
            analysis.mean_brpm > 6.0 && analysis.mean_brpm < 24.0,
            "BRPM {:.1} should be physiologically plausible",
            analysis.mean_brpm
        );
    }

    #[test]
    fn analyze_respiration_phase_intervals() {
        let params = RespSimulationParams {
            brpm: 12.0,
            dur_min: 1.0,
            fs: 50.0,
            noise_level: 0.0,
            ..Default::default()
        };
        let ts = generate_respiration_timeseries(&params);
        let analysis = analyze_respiration(&ts);
        assert!(!analysis.phase_peaks.inhalation_peak_indices.is_empty());
        assert!(
            !analysis.insp_peak_intervals_sec.is_empty()
                || analysis.phase_peaks.inhalation_peak_indices.len() < 2
        );
    }

    #[test]
    fn detect_respiration_peaks_empty() {
        let ts = RespTimeSeries {
            times: vec![],
            flow: vec![],
            volume: vec![],
            inhalation_peaks: vec![],
            exhalation_peaks: vec![],
        };
        let intervals = detect_respiration_peaks(&ts);
        assert!(intervals.peak_indices.is_empty());
    }
}
