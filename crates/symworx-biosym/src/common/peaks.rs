// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

use symworx_core::{Peak, PeakFinderBuilder};

use super::{intervals::IntervalSeries, signal::PhysiologySignal};

/// Alias for a detected peak from symworx-core signal processing.
pub type PhysiologyPeak = Peak;

/// Run peak detection using a configured finder builder.
pub fn detect_peaks(finder: PeakFinderBuilder<'_>) -> Vec<PhysiologyPeak> {
    finder.find()
}

/// Convert detected peaks into an [`IntervalSeries`].
pub fn peaks_to_intervals(peaks: &[PhysiologyPeak], fs: f64) -> IntervalSeries {
    let mut indices: Vec<usize> = peaks.iter().map(|p| p.index).collect();
    indices.sort_unstable();
    IntervalSeries::from_peak_indices(&indices, fs)
}

/// Detect peaks on a physiology signal with a custom finder preset.
pub fn detect_intervals(
    signal: &PhysiologySignal,
    finder: PeakFinderBuilder<'_>,
) -> IntervalSeries {
    let peaks = detect_peaks(finder);
    peaks_to_intervals(&peaks, signal.fs)
}

/// Indices of simple local maxima (3-point comparison), unfiltered.
pub fn local_maxima_indices(samples: &[f64]) -> Vec<usize> {
    if samples.len() < 3 {
        return vec![];
    }
    (1..samples.len() - 1)
        .filter(|&i| samples[i] > samples[i - 1] && samples[i] > samples[i + 1])
        .collect()
}
