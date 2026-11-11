// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Local-maximum peak finder on a slice.
//!
//! The decisions match `symworx-signal`'s `PeakFinder`: a sample is a candidate
//! when it is strictly greater than both neighbors, then height, neighbor
//! threshold, prominence, and minimum distance are applied in that order.
//! Minimum distance keeps the earlier peak.

use alloc::vec::Vec;

use crate::NpuError;

/// One accepted local maximum.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Peak {
    /// Index into the input slice.
    pub index: usize,
    /// Sample value at `index`.
    pub height: f64,
    /// Height above the higher of the two contour bases.
    pub prominence: f64,
}

/// Configured scan of one borrowed signal.
#[derive(Debug, Clone)]
pub struct PeakFinder<'a> {
    signal: &'a [f64],
    height: Option<f64>,
    prominence: Option<f64>,
    distance: Option<usize>,
    threshold: Option<f64>,
}

impl<'a> PeakFinder<'a> {
    /// Scan `signal`. Series shorter than 3 samples produce no peaks.
    pub fn new(signal: &'a [f64]) -> Self {
        Self {
            signal,
            height: None,
            prominence: None,
            distance: None,
            threshold: None,
        }
    }

    /// Keep candidates with `signal[i] >= h`.
    pub fn height(mut self, h: f64) -> Self {
        self.height = Some(h);
        self
    }

    /// Keep candidates whose prominence is at least `p`.
    pub fn prominence(mut self, p: f64) -> Self {
        self.prominence = Some(p);
        self
    }

    /// Drop a later peak that falls inside `samples` of the last kept peak.
    pub fn distance(mut self, samples: usize) -> Self {
        self.distance = Some(samples);
        self
    }

    /// Keep candidates that rise at least `t` above the higher neighbor.
    pub fn threshold(mut self, t: f64) -> Self {
        self.threshold = Some(t);
        self
    }

    /// Local maxima that pass the configured filters.
    pub fn find(self) -> Vec<Peak> {
        if self.signal.len() < 3 {
            return Vec::new();
        }
        let mut peaks = find_local_maxima(self.signal);

        if let Some(h) = self.height {
            peaks.retain(|&i| self.signal[i] >= h);
        }
        if let Some(th) = self.threshold {
            peaks.retain(|&i| {
                let neighbors = if i == 0 {
                    self.signal[i + 1]
                } else if i + 1 == self.signal.len() {
                    self.signal[i - 1]
                } else {
                    self.signal[i - 1].max(self.signal[i + 1])
                };
                self.signal[i] - neighbors >= th
            });
        }
        if let Some(p) = self.prominence {
            peaks.retain(|&i| prominence(self.signal, i) >= p);
        }
        if let Some(d) = self.distance {
            peaks = enforce_min_distance(&peaks, d);
        }

        peaks
            .into_iter()
            .map(|i| Peak {
                index: i,
                height: self.signal[i],
                prominence: prominence(self.signal, i),
            })
            .collect()
    }
}

/// Sample count for a refractory interval. Values that round below 1 become 1.
pub fn sample_distance(fs: f64, interval_sec: f64) -> Result<usize, NpuError> {
    if !fs.is_finite() || fs <= 0.0 {
        return Err(NpuError::InvalidInput("sample rate must be finite and positive"));
    }
    if !interval_sec.is_finite() || interval_sec <= 0.0 {
        return Err(NpuError::InvalidInput("minimum interval must be finite and positive"));
    }
    let samples = (interval_sec * fs).round();
    if !samples.is_finite() || samples > usize::MAX as f64 {
        return Err(NpuError::InvalidInput(
            "minimum interval does not fit a sample distance",
        ));
    }
    Ok((samples as usize).max(1))
}

fn find_local_maxima(signal: &[f64]) -> Vec<usize> {
    let mut candidates = Vec::new();
    for i in 1..signal.len() - 1 {
        if signal[i] > signal[i - 1] && signal[i] > signal[i + 1] {
            candidates.push(i);
        }
    }
    candidates
}

fn prominence(signal: &[f64], idx: usize) -> f64 {
    let left = find_base(signal, idx, -1);
    let right = find_base(signal, idx, 1);
    let base = signal[left].min(signal[right]);
    signal[idx] - base
}

fn find_base(signal: &[f64], start: usize, dir: isize) -> usize {
    let mut i = start as isize;
    let peak_val = signal[start];
    let mut base = start;
    let mut min_val = peak_val;
    let len = signal.len() as isize;

    while i >= 0 && i < len {
        let val = signal[i as usize];
        if val < min_val {
            min_val = val;
            base = i as usize;
        }
        if val > peak_val {
            break;
        }
        i += dir;
    }
    base
}

fn enforce_min_distance(peaks: &[usize], min_dist: usize) -> Vec<usize> {
    if peaks.is_empty() {
        return Vec::new();
    }
    let mut result = Vec::with_capacity(peaks.len());
    result.push(peaks[0]);
    for &p in &peaks[1..] {
        if p >= result[result.len() - 1] + min_dist {
            result.push(p);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_strict_local_maxima_and_skips_non_finite_samples() {
        let signal = [0.0, 1.0, 0.0, f64::NAN, 0.2, 2.0, 0.2];
        let peaks = PeakFinder::new(&signal).find();
        assert_eq!(peaks.len(), 2);
        assert_eq!(peaks[0].index, 1);
        assert_eq!(peaks[1].index, 5);
        assert!((peaks[1].height - 2.0).abs() < 1e-12);
        assert!(PeakFinder::new(&[1.0, 2.0]).find().is_empty());
    }

    #[test]
    fn height_prominence_and_distance_filter_in_host_order() {
        // Two close spikes, then a later small bump on a raised shelf.
        let mut signal = vec![0.0; 20];
        signal[3] = 1.0;
        signal[5] = 0.9;
        signal[12] = 0.4;
        for v in signal.iter_mut().skip(10) {
            *v += 0.2;
        }
        signal[12] = 0.4;

        let peaks = PeakFinder::new(&signal).height(0.5).prominence(0.5).distance(4).find();
        assert_eq!(peaks.len(), 1);
        assert_eq!(peaks[0].index, 3);
    }

    #[test]
    fn sample_distance_matches_a_rounded_refractory_window() {
        assert_eq!(sample_distance(100.0, 0.5).unwrap(), 50);
        assert_eq!(sample_distance(100.0, 0.001).unwrap(), 1);
        assert!(sample_distance(0.0, 0.5).is_err());
        assert!(sample_distance(100.0, f64::NAN).is_err());
    }
}
