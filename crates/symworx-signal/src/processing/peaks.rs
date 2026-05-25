// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

use ndarray::{Array1, ArrayView1};

#[derive(Debug, Clone, PartialEq)]
pub struct Peak {
    pub index: usize,
    pub height: f64,
    pub prominence: f64,
}

#[derive(Debug, Default, Clone)]
pub struct PeakFinder {
    height: Option<f64>,
    prominence: Option<f64>,
    distance: Option<usize>,
    threshold: Option<f64>,
}

impl PeakFinder {
    pub fn new(signal: ArrayView1<f64>) -> PeakFinderBuilder {
        PeakFinderBuilder {
            signal,
            config: PeakFinder::default(),
        }
    }

    // Internal config applicator
    fn apply_filters(&self, signal: ArrayView1<f64>, candidates: Vec<usize>) -> Vec<Peak> {
        let mut peaks = candidates;

        if let Some(h) = self.height {
            peaks.retain(|&i| signal[i] >= h);
        }

        if let Some(th) = self.threshold {
            peaks.retain(|&i| {
                let neighbors = if i == 0 { signal[i + 1] } else if i == signal.len() - 1 { signal[i - 1] } 
                else { signal[i - 1].max(signal[i + 1]) };
                signal[i] - neighbors >= th
            });
        }

        if let Some(p) = self.prominence {
            peaks = self.filter_by_prominence(signal, &peaks, p);
        }

        if let Some(d) = self.distance {
            peaks = enforce_min_distance(&peaks, d);
        }

        peaks.into_iter()
            .map(|i| Peak {
                index: i,
                height: signal[i],
                prominence: prominence(signal, i),
            })
            .collect()
    }

    fn filter_by_prominence(
        &self,
        signal: ArrayView1<f64>,
        candidates: &[usize],
        min_prom: f64,
    ) -> Vec<usize> {
        candidates
            .iter()
            .filter(|&&i| prominence(signal, i) >= min_prom)
            .copied()
            .collect()
    }
}

// Builder pattern
pub struct PeakFinderBuilder<'a> {
    signal: ArrayView1<'a, f64>,
    config: PeakFinder,
}

impl<'a> PeakFinderBuilder<'a> {
    pub fn height(mut self, h: f64) -> Self {
        self.config.height = Some(h);
        self
    }

    pub fn prominence(mut self, p: f64) -> Self {
        self.config.prominence = Some(p);
        self
    }

    pub fn distance(mut self, d: usize) -> Self {
        self.config.distance = Some(d);
        self
    }

    pub fn threshold(mut self, t: f64) -> Self {
        self.config.threshold = Some(t);
        self
    }

    pub fn find(self) -> Vec<Peak> {
        if self.signal.len() < 3 {
            return vec![];
        }

        let candidates = find_local_maxima(self.signal);
        self.config.apply_filters(self.signal, candidates)
    }
}

// Raw local maxima finder
fn find_local_maxima(signal: ArrayView1<f64>) -> Vec<usize> {
    let mut candidates = Vec::new();
    for i in 1..signal.len() - 1 {
        if signal[i] > signal[i - 1] && signal[i] > signal[i + 1] {
            candidates.push(i);
        }
    }
    candidates
}

// Prominence calculation
fn prominence(signal: ArrayView1<f64>, idx: usize) -> f64 {
    let left_base = find_base(signal, idx, -1);
    let right_base = find_base(signal, idx, 1);
    let base = signal[left_base].min(signal[right_base]);
    signal[idx] - base
}

fn find_base(signal: ArrayView1<f64>, start: usize, dir: isize) -> usize {
    let mut i = start as isize;
    let peak_val = signal[start];
    let mut base = start;
    let mut min_val = peak_val;

    while i >= 0 && i < signal.len() as isize {
        let val = signal[i as usize];
        if val < min_val {
            min_val = val;
            base = i as usize;
        }
        if val > peak_val {
            break; // crossed a higher contour
        }
        i += dir;
    }
    base
}

fn enforce_min_distance(peaks: &[usize], min_dist: usize) -> Vec<usize> {
    if peaks.is_empty() {
        return vec![];
    }
    let mut result = vec![peaks[0]];
    for &p in &peaks[1..] {
        if p >= result.last().unwrap() + min_dist {
            result.push(p);
        }
    }
    result
}
