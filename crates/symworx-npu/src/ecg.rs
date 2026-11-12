// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! First-cut R-peak preset.
//!
//! Band-limit a single lead, then take local maxima with a refractory gap.
//! This is the path to use when the R wave is the tall narrow peak. A
//! derivative-energy stage, search-back, and P/T picking are not in this preset.

use alloc::vec::Vec;

use crate::{
    BandpassSpec,
    NpuError,
    Peak,
    PeakFinder,
    sample_distance,
};

/// Thresholds for [`detect_ecg_r_peaks_with`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EcgDetectParams {
    /// Sample rate (Hz).
    pub fs: f64,
    /// Low cutoff (Hz).
    pub low_hz: f64,
    /// High cutoff (Hz).
    pub high_hz: f64,
    /// Section quality factor.
    pub q: f64,
    /// Cascaded 2nd-order sections. Zero is treated as one.
    pub stages: u8,
    /// Minimum time between accepted R peaks (seconds).
    pub min_interval_sec: f64,
    /// Minimum prominence after the bandpass.
    pub min_prominence: f64,
    /// Minimum sample value after the bandpass.
    pub min_height: f64,
}

impl EcgDetectParams {
    /// 5–15 Hz, two sections, q = 0.707, 0.2 s refractory, prominence 0.2.
    pub fn reference(fs: f64) -> Self {
        Self {
            fs,
            low_hz: 5.0,
            high_hz: 15.0,
            q: 0.707,
            stages: 2,
            min_interval_sec: 0.2,
            min_prominence: 0.2,
            min_height: 0.0,
        }
    }
}

/// R peaks with [`EcgDetectParams::reference`].
pub fn detect_ecg_r_peaks(signal: &[f64], fs: f64) -> Result<Vec<Peak>, NpuError> {
    detect_ecg_r_peaks_with(signal, &EcgDetectParams::reference(fs))
}

/// R peaks with caller thresholds. The bandpass runs before the finder.
pub fn detect_ecg_r_peaks_with(signal: &[f64], params: &EcgDetectParams) -> Result<Vec<Peak>, NpuError> {
    let distance = sample_distance(params.fs, params.min_interval_sec)?;
    let filtered = BandpassSpec {
        fs: params.fs,
        low_hz: params.low_hz,
        high_hz: params.high_hz,
        q: params.q,
        stages: params.stages,
    }
    .apply(signal)?;
    Ok(PeakFinder::new(&filtered)
        .distance(distance)
        .prominence(params.min_prominence)
        .height(params.min_height)
        .find())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qrs_train(fs: f64, period: f64, n_beats: usize) -> Vec<f64> {
        let n = ((n_beats as f64 + 0.5) * period * fs).round() as usize;
        let mut y: Vec<f64> = vec![0.0; n];
        let half = (0.02 * fs).round().max(1.0) as usize;
        for b in 0..n_beats {
            let c = ((b as f64 + 0.5) * period * fs).round() as usize;
            for k in 0..=half {
                let v = 1.0 - k as f64 / half as f64;
                if c >= k {
                    y[c - k] = y[c - k].max(v);
                }
                if c + k < n {
                    y[c + k] = y[c + k].max(v);
                }
            }
        }
        y
    }

    #[test]
    fn reference_preset_spaces_one_hertz_qrs_by_about_a_second() {
        let fs = 250.0;
        let y = qrs_train(fs, 1.0, 6);
        let peaks = detect_ecg_r_peaks(&y, fs).unwrap();
        assert_eq!(peaks.len(), 6);
        let intervals: Vec<f64> = peaks
            .windows(2)
            .map(|w| (w[1].index - w[0].index) as f64 / fs)
            .collect();
        let mean = intervals.iter().sum::<f64>() / intervals.len() as f64;
        assert!((mean - 1.0).abs() < 0.02, "mean interval {mean}");
    }

    #[test]
    fn refractory_window_collapses_a_close_pair() {
        let fs = 250.0;
        let mut y: Vec<f64> = qrs_train(fs, 1.0, 2);
        let extra = (0.5 * fs).round() as usize + (0.05 * fs).round() as usize;
        let half = (0.02 * fs).round() as usize;
        for k in 0..=half {
            let v = 0.8 - 0.8 * k as f64 / half as f64;
            if extra + k < y.len() {
                y[extra + k] = y[extra + k].max(v);
                y[extra - k] = y[extra - k].max(v);
            }
        }
        let mut params = EcgDetectParams::reference(fs);
        params.min_prominence = 0.05;
        let peaks = detect_ecg_r_peaks_with(&y, &params).unwrap();
        assert!(peaks.len() >= 2, "peaks {}", peaks.len());
        for w in peaks.windows(2) {
            let gap = (w[1].index - w[0].index) as f64 / fs;
            assert!(gap + 1e-9 >= 0.2, "gap {gap}");
        }
    }
}
