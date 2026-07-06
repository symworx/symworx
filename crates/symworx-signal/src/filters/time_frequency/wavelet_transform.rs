// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Wavelet Transform (Continuous Wavelet Transform - CWT)
//!
//! Useful for multi-resolution time-frequency analysis of non-stationary
//! signals (e.g., gait, heart rate variability, EEG, respiration).

use std::f64::consts::PI;

use ndarray::{Array1, Array2};

/// Common mother wavelets for CWT.
#[derive(Debug, Clone, Copy)]
pub enum WaveletType {
    /// Morlet wavelet (good for oscillatory signals)
    Morlet,
    /// Mexican Hat (Ricker) wavelet (good for transients)
    MexicanHat,
}

/// Result of a Continuous Wavelet Transform.
pub struct CwtResult {
    /// Scalogram: magnitude coefficients (scales × time)
    pub coefficients: Array2<f64>,
    /// Scales used
    pub scales: Array1<f64>,
    /// Corresponding frequencies (Hz)
    pub frequencies: Array1<f64>,
    /// Time axis
    pub times: Array1<f64>,
}

/// Continuous Wavelet Transform using Morlet or Mexican Hat wavelet.
pub fn cwt(
    signal: &[f64],
    fs: f64,
    wavelet: WaveletType,
    min_scale: f64,
    max_scale: f64,
    num_scales: usize,
) -> CwtResult {
    let n = signal.len();
    if n == 0 {
        return CwtResult {
            coefficients: Array2::zeros((0, 0)),
            scales: Array1::zeros(0),
            frequencies: Array1::zeros(0),
            times: Array1::zeros(0),
        };
    }

    let scales: Array1<f64> = Array1::linspace(min_scale, max_scale, num_scales);
    let mut coefficients = Array2::zeros((num_scales, n));

    let times: Array1<f64> = Array1::linspace(0.0, n as f64 / fs, n);

    for (i, &scale) in scales.iter().enumerate() {
        for t in 0..n {
            let mut sum = 0.0;
            for k in 0..n {
                let tau = (t as f64 - k as f64) / scale;
                let wavelet_val = match wavelet {
                    WaveletType::Morlet => {
                        let omega0 = 6.0; // standard value
                        let gauss = (-0.5 * tau * tau).exp();
                        let osc = (omega0 * tau).cos();
                        gauss * osc / (PI.sqrt() * scale.sqrt())
                    }
                    WaveletType::MexicanHat => {
                        let gauss = (-0.5 * tau * tau).exp();
                        (1.0 - tau * tau) * gauss / (PI.sqrt() * scale.sqrt())
                    }
                };
                sum += signal[k] * wavelet_val;
            }
            coefficients[[i, t]] = sum;
        }
    }

    // Approximate frequencies corresponding to scales
    let frequencies: Array1<f64> = scales.mapv(|s| fs / (4.0 * s)); // rough approximation

    CwtResult {
        coefficients: coefficients.mapv(|x| x.abs()), // return magnitude
        scales,
        frequencies,
        times,
    }
}

/// Convenience function for Wavelet transform (Morelet).
pub fn cwt_morlet(
    signal: &[f64],
    fs: f64,
    min_scale: f64,
    max_scale: f64,
    num_scales: usize,
) -> CwtResult {
    cwt(
        signal,
        fs,
        WaveletType::Morlet,
        min_scale,
        max_scale,
        num_scales,
    )
}

/// Convenience function for Wavelet transform (Mexican Hat).
pub fn cwt_mexhat(
    signal: &[f64],
    fs: f64,
    min_scale: f64,
    max_scale: f64,
    num_scales: usize,
) -> CwtResult {
    cwt(
        signal,
        fs,
        WaveletType::MexicanHat,
        min_scale,
        max_scale,
        num_scales,
    )
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cwt_basic() {
        let signal: Vec<f64> = (0..512)
            .map(|i| (2.0 * PI * 8.0 * i as f64 / 256.0).sin())
            .collect();

        let result = cwt_morlet(&signal, 256.0, 4.0, 64.0, 32);

        assert_eq!(result.coefficients.shape(), &[32, 512]);
        assert!(result.frequencies[0] > result.frequencies[result.frequencies.len() - 1]);
    }

    #[test]
    fn test_cwt_empty() {
        let result = cwt_morlet(&[], 100.0, 1.0, 10.0, 5);
        assert_eq!(result.coefficients.shape()[0], 0);
    }
}
