// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Short-Time Fourier Transform (STFT)
//!
//! Computes spectrograms for time-frequency analysis of signals.
//! Particularly useful for physiological signals (gait, respiration, EMG, etc.).

use ndarray::{Array1, Array2, s};
use rustfft::{FftPlanner, num_complex::Complex};

/// Window function for STFT.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowType {
    Hann,
    Hamming,
    Rectangular,
}

/// Result of an STFT computation.
pub struct StftResult {
    /// Spectrogram: magnitude (time × frequency)
    pub spectrogram: Array2<f64>,
    /// Frequency bins (Hz)
    pub frequencies: Array1<f64>,
    /// Time centers of each window (seconds)
    pub times: Array1<f64>,
    /// Sampling frequency used
    pub fs: f64,
}

impl StftResult {
    /// Returns the spectrogram in dB scale (log magnitude).
    pub fn to_db(&self, min_db: f64) -> Array2<f64> {
        self.spectrogram.mapv(|x| {
            let db = 20.0 * x.log10();
            db.max(min_db)
        })
    }
}

/// Computes the Short-Time Fourier Transform of a signal.
pub fn stft(
    signal: &[f64],
    fs: f64,
    window_length: usize,
    overlap: usize,
    window_type: WindowType,
) -> StftResult {
    assert!(window_length > 1, "Window length must be > 1");
    assert!(overlap < window_length, "Overlap must be less than window length");

    let hop = window_length - overlap;
    let n_windows = (signal.len() - window_length) / hop + 1;

    if n_windows < 1 {
        return StftResult {
            spectrogram: Array2::zeros((0, 0)),
            frequencies: Array1::zeros(0),
            times: Array1::zeros(0),
            fs,
        };
    }

    let n_freq = window_length / 2 + 1;

    let mut spectrogram = Array2::zeros((n_windows, n_freq));
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(window_length);

    // Generate window
    let window: Vec<f64> = match window_type {
        WindowType::Hann => (0..window_length)
            .map(|i| 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (window_length - 1) as f64).cos()))
            .collect(),
        WindowType::Hamming => (0..window_length)
            .map(|i| 0.54 - 0.46 * (2.0 * std::f64::consts::PI * i as f64 / (window_length - 1) as f64).cos())
            .collect(),
        WindowType::Rectangular => vec![1.0; window_length],
    };

    for (i, win_start) in (0..n_windows).map(|i| i * hop).enumerate() {
        let segment: Vec<Complex<f64>> = (0..window_length)
            .map(|j| {
                let idx = win_start + j;
                let val = if idx < signal.len() { signal[idx] } else { 0.0 };
                Complex::new(val * window[j], 0.0)
            })
            .collect();

        let mut fft_output = segment.clone();
        fft.process(&mut fft_output);

        // Take first half (positive frequencies) and compute magnitude
        for k in 0..n_freq {
            let mag = fft_output[k].norm() / window_length as f64;
            spectrogram[[i, k]] = mag;
        }
    }

    // Frequency and time axes
    let frequencies: Array1<f64> = Array1::linspace(0.0, fs / 2.0, n_freq);
    let times: Array1<f64> = Array1::linspace(
        window_length as f64 / (2.0 * fs),
        (signal.len() as f64) / fs,
        n_windows,
    );

    StftResult {
        spectrogram,
        frequencies,
        times,
        fs,
    }
}

// ——— Tests ———

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stft_basic() {
        let signal: Vec<f64> = (0..1024)
            .map(|i| (2.0 * std::f64::consts::PI * 10.0 * i as f64 / 256.0).sin())
            .collect();

        let result = stft(&signal, 256.0, 128, 64, WindowType::Hann);

        assert!(result.spectrogram.shape()[0] > 5);
        assert_eq!(result.spectrogram.shape()[1], 65); // 128/2 + 1
        assert!(result.frequencies[result.frequencies.len() - 1] > 100.0);
    }

    #[test]
    fn test_stft_empty() {
        let result = stft(&[], 100.0, 64, 32, WindowType::Hann);
        assert_eq!(result.spectrogram.shape()[0], 0);
    }
}
