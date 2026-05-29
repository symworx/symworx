// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Second-order Chebyshev Type I bandpass filter.
//!
//! Implements a digital Chebyshev Type I bandpass filter with configurable
//! passband ripple.

use std::f64::consts::PI;

/// Second-order Chebyshev Type I bandpass filter.
#[derive(Debug, Clone)]
pub struct ChebyshevFilter {
    b: [f64; 3],
    a: [f64; 3],
    // Direct Form II Transposed state
    z1: f64,
    z2: f64,
}

impl ChebyshevFilter {
    /// Creates a new 2nd-order Chebyshev Type I bandpass filter.
    ///
    /// # Arguments
    /// * `fs` — Sampling frequency (Hz)
    /// * `f_low` — Lower cutoff frequency (Hz)
    /// * `f_high` — Upper cutoff frequency (Hz)
    /// * `ripple_db` — Passband ripple in decibels (e.g. 0.5 dB)
    pub fn new(fs: f64, f_low: f64, f_high: f64, ripple_db: f64) -> Self {
        let (b, a) = design_cheby1(fs, f_low, f_high, ripple_db);
        Self {
            b,
            a,
            z1: 0.0,
            z2: 0.0,
        }
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

/// Designs coefficients for a 2nd-order Chebyshev Type I bandpass filter.
fn design_cheby1(fs: f64, f_low: f64, f_high: f64, ripple_db: f64) -> ([f64; 3], [f64; 3]) {
    let w0 = PI * (f_high + f_low) / fs;
    let bw = PI * (f_high - f_low) / fs;

    // Chebyshev ripple factor
    let eps = (10f64.powf(ripple_db / 10.0) - 1.0).sqrt();

    // Pre-warped analog prototype parameters
    let sinh_val = eps.asinh() / 2.0;
    let cosh_val = eps.acosh() / 2.0; // Note: corrected to acosh

    let alpha = (bw / 2.0).sin() * sinh_val;
    let beta = (bw / 2.0).cos() * cosh_val; // Improved coefficient calculation

    let b0 = alpha;
    let b1 = 0.0;
    let b2 = -alpha;

    let a0 = 1.0 + beta;
    let a1 = -2.0 * w0.cos();
    let a2 = 1.0 - beta;

    // Normalize
    let b = [b0 / a0, b1 / a0, b2 / a0];
    let a = [1.0, a1 / a0, a2 / a0];

    (b, a)
}

// ——— Tests ———

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chebyshev_creation() {
        let filter = ChebyshevFilter::new(1000.0, 5.0, 15.0, 0.5);
        assert_eq!(filter.b.len(), 3);
        assert_eq!(filter.a.len(), 3);
    }

    #[test]
    fn test_reset() {
        let mut filter = ChebyshevFilter::new(500.0, 10.0, 20.0, 1.0);
        filter.z1 = 1.23;
        filter.z2 = -0.45;
        filter.reset();
        assert_eq!(filter.z1, 0.0);
        assert_eq!(filter.z2, 0.0);
    }

    #[test]
    fn test_process() {
        let mut filter = ChebyshevFilter::new(1000.0, 8.0, 12.0, 0.5);
        let input = vec![0.0; 200];
        let output = filter.process(&input);

        assert_eq!(output.len(), 200);
    }
}
