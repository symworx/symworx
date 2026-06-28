// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Hilbert Transform and Analytic Signal.
//!
//! The Hilbert transform is used to compute the instantaneous amplitude,
//! phase, and frequency of signals — especially useful in physiological
//! signal analysis (e.g., EMG, EEG, respiration, gait).

use ndarray::Array1;
use num_complex::Complex;
use rustfft::{
    FftPlanner,
    num_complex::Complex as RustFftComplex,
};

/// Computes the Hilbert transform of a real-valued signal using FFT.
///
/// Returns the analytic signal (complex-valued).
pub fn hilbert(signal: &[f64]) -> Array1<Complex<f64>> {
    use rustfft::{
        FftPlanner,
        num_complex::Complex,
    };

    let n = signal.len();
    if n == 0 {
        return Array1::from_vec(vec![]);
    }

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n);

    // Convert to complex
    let mut buffer: Vec<Complex<f64>> = signal.iter().map(|&x| Complex::new(x, 0.0)).collect();

    // Forward FFT
    fft.process(&mut buffer);

    // Create Hilbert multiplier in frequency domain
    let mut hilbert_spec = vec![Complex::new(0.0, 0.0); n];

    hilbert_spec[0] = buffer[0]; // DC component

    for i in 1..(n / 2) {
        hilbert_spec[i] = buffer[i] * Complex::new(2.0, 0.0);
    }

    if n.is_multiple_of(2) {
        hilbert_spec[n / 2] = buffer[n / 2]; // Nyquist
    }

    // Inverse FFT
    let ifft = planner.plan_fft_inverse(n);
    ifft.process(&mut hilbert_spec);

    // rustfft IFFT is unnormalized (forward+inverse gives n*original), so scale
    let scale = 1.0 / n as f64;
    for c in &mut hilbert_spec {
        *c = *c * scale;
    }

    // Convert back to ndarray
    Array1::from_vec(hilbert_spec)
}

/// Computes the analytic signal, instantaneous amplitude, and phase.
pub struct AnalyticSignal {
    /// Complex analytic signal
    pub analytic: Array1<Complex<f64>>,
    /// Instantaneous amplitude (envelope)
    pub amplitude: Array1<f64>,
    /// Instantaneous phase (radians)
    pub phase: Array1<f64>,
    /// Instantaneous frequency (Hz) — requires sampling rate
    pub frequency: Option<Array1<f64>>,
}

impl AnalyticSignal {
    /// Compute analytic signal from real signal.
    pub fn from_signal(signal: &[f64]) -> Self {
        let analytic = hilbert(signal);

        let amplitude = analytic.mapv(|z| z.norm());
        let phase = analytic.mapv(|z| z.arg());

        Self {
            analytic,
            amplitude,
            phase,
            frequency: None,
        }
    }

    /// Compute instantaneous frequency (requires sampling rate).
    pub fn with_frequency(mut self, fs: f64) -> Self {
        let n = self.phase.len();
        if n < 2 {
            self.frequency = Some(Array1::zeros(n));
            return self;
        }

        let mut freq = Vec::with_capacity(n);
        freq.push(0.0); // First point undefined

        for i in 1..n {
            let mut dphase = self.phase[i] - self.phase[i - 1];
            // Unwrap phase jumps
            if dphase > std::f64::consts::PI {
                dphase -= 2.0 * std::f64::consts::PI;
            } else if dphase < -std::f64::consts::PI {
                dphase += 2.0 * std::f64::consts::PI;
            }
            let inst_freq = (dphase * fs) / (2.0 * std::f64::consts::PI);
            freq.push(inst_freq);
        }

        self.frequency = Some(Array1::from_vec(freq));
        self
    }
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hilbert_basic() {
        let signal = vec![0.0f64; 256];
        let analytic = hilbert(&signal);
        assert_eq!(analytic.len(), 256);
    }

    #[test]
    fn test_analytic_signal() {
        // Simple sine wave
        let t: Vec<f64> = (0..512).map(|i| i as f64 / 512.0 * 10.0).collect();
        let signal: Vec<f64> = t
            .iter()
            .map(|&x| (2.0 * std::f64::consts::PI * 5.0 * x).sin())
            .collect();

        let analytic = AnalyticSignal::from_signal(&signal);
        let with_freq = analytic.with_frequency(512.0);

        assert!(with_freq.amplitude.iter().all(|&a| a > 0.9 && a < 1.1)); // amplitude ≈ 1.0
        assert!(with_freq.frequency.is_some());
    }
}
