// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Welch power spectral density (averaged modified periodograms).

use ndarray::Array1;
use rustfft::{
    FftPlanner,
    num_complex::Complex,
};

use super::stft::{
    WindowType,
    window_samples,
};

/// Options for [`welch`].
#[derive(Debug, Clone, PartialEq)]
pub struct WelchConfig {
    /// Segment length. `None` → `min(256, n)`.
    pub nperseg: Option<usize>,
    /// Overlap in samples. `None` → `nperseg / 2`.
    pub overlap: Option<usize>,
    /// Taper applied to each segment.
    pub window: WindowType,
}

impl Default for WelchConfig {
    fn default() -> Self {
        Self {
            nperseg: None,
            overlap: None,
            window: WindowType::Hann,
        }
    }
}

/// Onesided PSD from [`welch`].
#[derive(Debug, Clone)]
pub struct WelchResult {
    /// Frequencies in Hz, `0 ..= Nyquist` (`nperseg / 2 + 1` bins).
    pub frequencies: Array1<f64>,
    /// Power spectral density (units² / Hz).
    pub psd: Array1<f64>,
    /// Sampling frequency used.
    pub fs: f64,
    /// Number of averaged segments.
    pub n_segments: usize,
}

fn empty_result(fs: f64) -> WelchResult {
    WelchResult {
        frequencies: Array1::zeros(0),
        psd: Array1::zeros(0),
        fs,
        n_segments: 0,
    }
}

/// Welch PSD with default Hann, 50% overlap, `nperseg = min(256, n)`.
pub fn welch_default(signal: &[f64], fs: f64) -> WelchResult {
    welch(signal, fs, &WelchConfig::default())
}

/// Welch estimate of the onesided power spectral density.
///
/// Density scaling includes window-power compensation. Bins strictly between
/// DC and Nyquist are doubled (onesided). Empty or non-positive `fs` → empty
/// axes. If `n < 2`, empty. A requested `nperseg` larger than `n` is clamped
/// to `n` (single segment).
pub fn welch(signal: &[f64], fs: f64, cfg: &WelchConfig) -> WelchResult {
    let n = signal.len();
    if n < 2 || !fs.is_finite() || fs <= 0.0 {
        return empty_result(fs);
    }

    let mut nperseg = cfg.nperseg.unwrap_or(256.min(n)).max(2);
    if nperseg > n {
        nperseg = n;
    }
    let overlap = cfg.overlap.unwrap_or(nperseg / 2).min(nperseg.saturating_sub(1));
    let hop = nperseg - overlap; // overlap ≤ nperseg − 1 ⇒ hop ≥ 1
    let n_segments = (n - nperseg) / hop + 1;

    let window = window_samples(cfg.window, nperseg);
    let wpow: f64 = window.iter().map(|w| w * w).sum();
    if wpow <= 0.0 {
        return empty_result(fs);
    }
    let scale = 1.0 / (fs * wpow);

    let n_freq = nperseg / 2 + 1;
    let mut acc = vec![0.0; n_freq];
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(nperseg);

    for s in 0..n_segments {
        let start = s * hop;
        let mut buf: Vec<Complex<f64>> = (0..nperseg)
            .map(|j| {
                let idx = start + j;
                let v = if idx < n { signal[idx] } else { 0.0 };
                Complex::new(v * window[j], 0.0)
            })
            .collect();
        fft.process(&mut buf);
        for k in 0..n_freq {
            let p = buf[k].norm_sqr() * scale;
            acc[k] += p;
        }
    }

    let inv_seg = 1.0 / n_segments as f64;
    for v in &mut acc {
        *v *= inv_seg;
    }
    // Onesided: double interior bins (not DC; not Nyquist when nperseg even).
    let last = n_freq - 1;
    let interior_end = if nperseg.is_multiple_of(2) { last } else { n_freq };
    for p in acc.iter_mut().take(interior_end).skip(1) {
        *p *= 2.0;
    }

    let frequencies = Array1::linspace(0.0, fs / 2.0, n_freq);
    WelchResult {
        frequencies,
        psd: Array1::from(acc),
        fs,
        n_segments,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_peak_near_known_freq() {
        let fs = 256.0;
        let f0 = 32.0;
        let n = 2048;
        let signal: Vec<f64> = (0..n)
            .map(|i| (2.0 * std::f64::consts::PI * f0 * i as f64 / fs).sin())
            .collect();
        let r = welch_default(&signal, fs);
        assert!(r.n_segments >= 2);
        assert_eq!(r.frequencies.len(), r.psd.len());
        let k = r
            .psd
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();
        assert!((r.frequencies[k] - f0).abs() < 2.0, "peak at {} Hz", r.frequencies[k]);
    }

    #[test]
    fn empty_and_bad_fs() {
        let e = welch_default(&[], 100.0);
        assert_eq!(e.frequencies.len(), 0);
        let e = welch_default(&[1.0, 2.0, 3.0], 0.0);
        assert_eq!(e.n_segments, 0);
    }

    #[test]
    fn short_signal_one_segment() {
        let signal: Vec<f64> = (0..64).map(|i| (i as f64).sin()).collect();
        let r = welch(
            &signal,
            64.0,
            &WelchConfig {
                nperseg: Some(256),
                overlap: None,
                window: WindowType::Hann,
            },
        );
        assert_eq!(r.n_segments, 1);
        assert_eq!(r.frequencies.len(), 64 / 2 + 1);
    }
}
