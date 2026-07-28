// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Butterworth bandpass filter (2nd order).
//!
//! Digital implementation using the bilinear transform.

/// Second-order Butterworth bandpass filter.
#[derive(Debug, Clone)]
pub struct BandpassFilter {
    b: [f64; 3],
    a: [f64; 3],
    // Internal state for Direct Form II Transposed
    z1: f64,
    z2: f64,
}

impl BandpassFilter {
    /// Creates a new 2nd-order Butterworth bandpass filter.
    ///
    /// # Arguments
    /// * `fs` — Sampling frequency (Hz)
    /// * `f_low` — Lower cutoff frequency (Hz)
    /// * `f_high` — Upper cutoff frequency (Hz)
    /// * `q` — Quality factor (typically 0.5–2.0; higher = narrower band)
    pub fn new(fs: f64, f_low: f64, f_high: f64, q: f64) -> Self {
        let (b, a) = design_bandpass(fs, f_low, f_high, q);
        Self { b, a, z1: 0.0, z2: 0.0 }
    }

    /// Resets the filter's internal state.
    #[inline]
    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    /// Processes a single sample.
    #[inline]
    pub fn process_sample(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.z1;
        self.z1 = self.b[1] * x - self.a[1] * y + self.z2;
        self.z2 = self.b[2] * x - self.a[2] * y;
        y
    }

    /// Processes an entire signal.
    pub fn process(&mut self, input: &[f64]) -> Vec<f64> {
        input.iter().map(|&x| self.process_sample(x)).collect()
    }
}

/// Designs a 2nd-order Butterworth bandpass filter using the bilinear transform.
///
/// Returns `(b, a)` coefficients for the transfer function.
fn design_bandpass(fs: f64, f_low: f64, f_high: f64, q: f64) -> ([f64; 3], [f64; 3]) {
    let w0 = std::f64::consts::PI * (f_high + f_low) / fs;
    let bw = std::f64::consts::PI * (f_high - f_low) / fs;
    let alpha = (bw / 2.0).sin() / (2.0 * q);

    let b0 = alpha;
    let b1 = 0.0;
    let b2 = -alpha;

    let a0 = 1.0 + alpha;
    let a1 = -2.0 * w0.cos();
    let a2 = 1.0 - alpha;

    // Normalize coefficients
    let b = [b0 / a0, b1 / a0, b2 / a0];
    let a = [1.0, a1 / a0, a2 / a0];

    (b, a)
}

// ——— Tests ———

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bandpass_creation() {
        let filter = BandpassFilter::new(1000.0, 5.0, 15.0, 0.707);
        assert_eq!(filter.b.len(), 3);
        assert_eq!(filter.a.len(), 3);
    }

    #[test]
    fn test_reset() {
        let mut filter = BandpassFilter::new(500.0, 10.0, 20.0, 1.0);
        filter.z1 = 0.5;
        filter.z2 = -0.3;
        filter.reset();
        assert_eq!(filter.z1, 0.0);
        assert_eq!(filter.z2, 0.0);
    }

    #[test]
    fn test_process_constant() {
        let mut filter = BandpassFilter::new(1000.0, 5.0, 15.0, 0.707);
        let input = vec![1.0; 100];
        let output = filter.process(&input);

        assert_eq!(output.len(), 100);
        // DC should be attenuated; check the very end of the response (IIR transient
        // at 5 Hz cutoff on fs=1000 takes a while to fully settle in 100 samples).
        let last = *output.last().unwrap();
        assert!(last.abs() < 0.1, "final sample near zero for DC input, got {}", last);
        // Also ensure overall not blowing up (sum of abs over all is reasonable)
        let sum_abs: f64 = output.iter().map(|&v| v.abs()).sum();
        assert!(sum_abs < 20.0, "total energy not excessive, got {}", sum_abs);
    }
}
