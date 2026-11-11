// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Gait event presets on the scalar peak finder.
//!
//! Numbers match `symworx-biosym` stride detection: a 0.5 s floor, prominence
//! 0.08, and the 0.2–6 Hz quality band (q = 0.707, two sections). These peaks
//! are local maxima on one reference channel. Heel-strike and toe-off are a
//! later preset.

use alloc::vec::Vec;

use crate::{
    BandpassSpec,
    NpuError,
    Peak,
    PeakFinder,
    sample_distance,
};

/// Same labels as `GaitSignalQuality`. `High` uses the reference thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaitQuality {
    /// Mild bandpass, height 0.05, prominence 0.08, 0.5 s floor.
    Reference,
    /// Same thresholds as [`GaitQuality::Reference`].
    High,
    /// Bandpass, prominence 0.10, height 0.0, 0.5 s floor.
    Moderate,
    /// Bandpass, prominence 0.15, height 0.20, 0.6 s floor.
    Poor,
}

/// Peaks with the no-bandpass stride preset (`detect_gait_strides`).
pub fn detect_gait_peaks(signal: &[f64], fs: f64) -> Result<Vec<Peak>, NpuError> {
    find_with(signal, fs, 0.5, 0.08, 0.0, None)
}

/// Peaks after the quality preset's bandpass.
pub fn detect_gait_peaks_with_quality(signal: &[f64], fs: f64, quality: GaitQuality) -> Result<Vec<Peak>, NpuError> {
    let (interval, prominence, height) = match quality {
        GaitQuality::Reference | GaitQuality::High => (0.5, 0.08, 0.05),
        GaitQuality::Moderate => (0.5, 0.10, 0.0),
        GaitQuality::Poor => (0.6, 0.15, 0.20),
    };
    let band = BandpassSpec {
        fs,
        low_hz: 0.2,
        high_hz: 6.0,
        q: 0.707,
        stages: 2,
    };
    find_with(signal, fs, interval, prominence, height, Some(band))
}

fn find_with(
    signal: &[f64],
    fs: f64,
    interval_sec: f64,
    prominence: f64,
    height: f64,
    band: Option<BandpassSpec>,
) -> Result<Vec<Peak>, NpuError> {
    let distance = sample_distance(fs, interval_sec)?;
    let filtered;
    let samples = if let Some(spec) = band {
        filtered = spec.apply(signal)?;
        filtered.as_slice()
    } else {
        signal
    };
    Ok(PeakFinder::new(samples)
        .distance(distance)
        .prominence(prominence)
        .height(height)
        .find())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step_humps(fs: f64, period: f64, n_peaks: usize, amp: f64) -> Vec<f64> {
        let sigma = 0.08 * fs;
        let last = period * n_peaks as f64;
        let n = (last * fs).round() as usize + 1;
        let mut y = vec![0.0; n];
        for k in 0..n_peaks {
            let center = (0.5 + k as f64) * period * fs;
            let i0 = (center - 4.0 * sigma).floor().max(0.0) as usize;
            let i1 = ((center + 4.0 * sigma).ceil() as usize).min(n);
            for (i, sample) in y.iter_mut().enumerate().take(i1).skip(i0) {
                let d = i as f64 - center;
                *sample += amp * (-0.5 * (d / sigma).powi(2)).exp();
            }
        }
        y
    }

    #[test]
    fn raw_preset_keeps_half_second_humps() {
        let fs = 100.0;
        let y = step_humps(fs, 0.5, 6, 1.0);
        let peaks = detect_gait_peaks(&y, fs).unwrap();
        assert_eq!(peaks.len(), 6);
        for w in peaks.windows(2) {
            assert_eq!(w[1].index - w[0].index, 50);
        }
    }

    #[test]
    fn reference_band_keeps_the_same_step_count() {
        let fs = 100.0;
        let y = step_humps(fs, 0.5, 6, 1.0);
        let peaks = detect_gait_peaks_with_quality(&y, fs, GaitQuality::Reference).unwrap();
        let gaps: Vec<usize> = peaks.windows(2).map(|w| w[1].index - w[0].index).collect();
        let steady = gaps.iter().filter(|&&gap| gap == 50).count();
        assert!(steady >= 3, "gaps {gaps:?}");
    }

    #[test]
    fn poor_preset_keeps_fewer_peaks_than_reference() {
        let fs = 100.0;
        let y = step_humps(fs, 0.5, 6, 1.0);
        let reference = detect_gait_peaks_with_quality(&y, fs, GaitQuality::Reference).unwrap();
        let poor = detect_gait_peaks_with_quality(&y, fs, GaitQuality::Poor).unwrap();
        assert!(reference.len() > 1);
        assert!(poor.len() < reference.len());
    }
}
