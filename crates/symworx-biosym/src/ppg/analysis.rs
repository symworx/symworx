// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

use symworx_core::PeakFinderBuilder;

use super::{
    PPGSignalQuality,
    PPGTimeSeries,
    processing::ppg_processing_for_quality,
};
use crate::common::{
    HrvMetrics,
    IntervalSeries,
    PhysiologyProcessingParams,
    PhysiologySignal,
    PhysiologySummary,
    apply_peak_overrides,
    compute_hrv_metrics,
    detect_intervals,
    preprocess_signal,
    summarize_signal,
};

const PPG_BASE_MIN_INTERVAL_SEC: f64 = 0.4;
const PPG_BASE_PROMINENCE: f64 = 0.12;
const PPG_BASE_HEIGHT: f64 = 0.3;

/// Results of PPG waveform analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct PpgAnalysis {
    pub summary: PhysiologySummary,
    pub intervals: IntervalSeries,
    pub mean_hr_bpm: f64,
    pub hrv: HrvMetrics,
}

/// Build a [`PhysiologySignal`] from a PPG time series.
pub fn ppg_signal(ts: &PPGTimeSeries) -> PhysiologySignal {
    PhysiologySignal::with_times(250.0, ts.values.clone(), ts.times.clone())
}

/// PPG peak finder with optional processing overrides (~150 bpm upper bound).
pub fn ppg_peak_finder<'a>(
    signal: &'a PhysiologySignal,
    processing: &PhysiologyProcessingParams,
) -> PeakFinderBuilder<'a> {
    apply_peak_overrides(
        PeakFinderBuilder::from_slice(&signal.samples),
        signal.fs,
        PPG_BASE_MIN_INTERVAL_SEC,
        PPG_BASE_PROMINENCE,
        PPG_BASE_HEIGHT,
        &processing.peaks,
    )
}

/// Detect systolic peaks (no bandpass).
pub fn detect_ppg_peaks(ts: &PPGTimeSeries) -> IntervalSeries {
    detect_ppg_peaks_with(ts, &PhysiologyProcessingParams::none())
        .expect("default detection has no bandpass")
}

/// Detect peaks with optional bandpass and peak overrides.
pub fn detect_ppg_peaks_with(
    ts: &PPGTimeSeries,
    processing: &PhysiologyProcessingParams,
) -> Result<IntervalSeries, &'static str> {
    let signal = preprocess_signal(ppg_signal(ts), processing)?;
    Ok(detect_intervals(
        &signal,
        ppg_peak_finder(&signal, processing),
    ))
}

/// Summarize a PPG recording (raw waveform, no filtering).
pub fn summarize_ppg(ts: &PPGTimeSeries) -> PhysiologySummary {
    summarize_signal(&ppg_signal(ts))
}

/// Full analysis without bandpass (backward compatible).
pub fn analyze_ppg(ts: &PPGTimeSeries) -> PpgAnalysis {
    analyze_ppg_with(ts, &PhysiologyProcessingParams::none())
        .expect("default analysis has no bandpass")
}

/// Full analysis with explicit processing parameters.
pub fn analyze_ppg_with(
    ts: &PPGTimeSeries,
    processing: &PhysiologyProcessingParams,
) -> Result<PpgAnalysis, &'static str> {
    let signal = preprocess_signal(ppg_signal(ts), processing)?;
    let summary = summarize_signal(&signal);
    let intervals = detect_intervals(&signal, ppg_peak_finder(&signal, processing));

    let mean_hr_bpm = if intervals.intervals_sec.is_empty() {
        intervals.mean_rate_over_window(summary.duration_sec)
    } else {
        intervals.mean_instantaneous_rate()
    };

    let hrv = compute_hrv_metrics(&intervals.intervals_sec);

    Ok(PpgAnalysis {
        summary,
        intervals,
        mean_hr_bpm,
        hrv,
    })
}

/// Analysis using quality-preset bandpass and peak settings.
pub fn analyze_ppg_with_quality(ts: &PPGTimeSeries, quality: PPGSignalQuality) -> PpgAnalysis {
    analyze_ppg_with(ts, &ppg_processing_for_quality(quality))
        .expect("quality presets use valid bandpass cutoffs")
}

#[cfg(test)]
mod tests {
    use super::{
        PPGNoiseConfig,
        generate_ppg_timeseries,
        *,
    };

    fn synthetic_sine_ppg(fs: f64, duration_sec: f64) -> PPGTimeSeries {
        let n = (duration_sec * fs).round() as usize;
        let mut times = Vec::with_capacity(n);
        let mut values = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f64 / fs;
            times.push(t);
            values.push(
                (2.0 * std::f64::consts::PI * 1.0 * t).sin()
                    + 0.5 * (2.0 * std::f64::consts::PI * 2.0 * t).sin(),
            );
        }
        PPGTimeSeries {
            times,
            values,
            systolic_peaks: vec![],
            diastolic_peaks: vec![],
        }
    }

    #[test]
    fn summarize_ppg_basic() {
        let ts = synthetic_sine_ppg(100.0, 1.0);
        let summary = summarize_ppg(&ts);
        assert!((summary.duration_sec - 1.0).abs() < 1e-6);
        assert!(summary.mean.is_finite());
        assert!(summary.std_dev.is_finite());
    }

    #[test]
    fn detect_ppg_peaks_on_sine() {
        let ts = synthetic_sine_ppg(100.0, 1.0);
        let intervals = detect_ppg_peaks(&ts);
        assert!(
            !intervals.peak_indices.is_empty(),
            "expected peaks on synthetic sine"
        );
        assert_eq!(intervals.peak_times.len(), intervals.peak_indices.len());
    }

    #[test]
    fn analyze_ppg_generated_timeseries() {
        let fs = 250.0;
        let rr_intervals = vec![0.85; 20];
        let beat_duration = 0.9;
        let params = (1.0, 0.2, 0.03, 0.35, 0.45, 0.06);
        let noise = PPGNoiseConfig::default();
        let ts = generate_ppg_timeseries(
            0.0,
            &rr_intervals,
            rr_intervals.len(),
            beat_duration,
            fs,
            params,
            &noise,
        );

        let analysis = analyze_ppg(&ts);
        let duration = analysis.summary.duration_sec;
        let expected_hr = 60.0 * ts.systolic_peaks.len() as f64 / duration;
        assert!(
            (analysis.mean_hr_bpm - expected_hr).abs() < 12.0,
            "mean HR {:.1} should be near {:.1} (from generation systolic peaks)",
            analysis.mean_hr_bpm,
            expected_hr
        );
    }

    #[test]
    fn analyze_ppg_with_bandpass_reference_quality() {
        let fs = 250.0;
        let noise = PPGNoiseConfig {
            global_noise_std: 0.08,
            ..Default::default()
        };
        let ts = generate_ppg_timeseries(
            0.0,
            &[0.85; 15],
            15,
            0.9,
            fs,
            (1.0, 0.2, 0.03, 0.35, 0.45, 0.06),
            &noise,
        );

        let filtered = analyze_ppg_with_quality(&ts, PPGSignalQuality::Reference);
        assert!(
            !filtered.intervals.peak_indices.is_empty(),
            "bandpass + reference preset should detect beats on noisy PPG"
        );
        assert!(
            filtered.mean_hr_bpm > 40.0 && filtered.mean_hr_bpm < 120.0,
            "HR {:.1} should be physiologically plausible",
            filtered.mean_hr_bpm
        );
    }

    #[test]
    fn analyze_ppg_hrv_on_variable_rr() {
        let fs = 250.0;
        let rr_intervals: Vec<f64> = (0..15)
            .map(|i| 0.8 + 0.05 * ((i as f64) * 0.5).sin())
            .collect();
        let ts = generate_ppg_timeseries(
            0.0,
            &rr_intervals,
            rr_intervals.len(),
            0.9,
            fs,
            (1.0, 0.2, 0.03, 0.35, 0.45, 0.06),
            &PPGNoiseConfig::default(),
        );
        let analysis = analyze_ppg(&ts);
        assert!(analysis.hrv.sdnn_sec.is_some());
        assert!(analysis.hrv.rmssd_sec.is_some());
    }

    #[test]
    fn detect_ppg_peaks_empty_signal() {
        let ts = PPGTimeSeries {
            times: vec![],
            values: vec![],
            systolic_peaks: vec![],
            diastolic_peaks: vec![],
        };
        let intervals = detect_ppg_peaks(&ts);
        assert!(intervals.peak_indices.is_empty());
    }
}
