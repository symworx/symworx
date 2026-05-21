// symworx/crates/symworx-signal/src/filters/linear/chebyshev.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use std::f64::consts::PI;

// ==========================================================
// Second order Chebyshev bandpass filter
// ==========================================================
/// Second‑order Chebyshev Type I bandpass filter
pub struct ChebyshevFilter {
    a: [f64; 3],
    b: [f64; 3],
    z1: f64,
    z2: f64,
}

impl ChebyshevFilter {
    /// fs = sample rate
    /// f_low, f_high = cutoff frequencies
    /// ripple_db = passband ripple in dB (e.g., 0.5)
    pub fn new(fs: f64, f_low: f64, f_high: f64, ripple_db: f64) -> Self {
        let (b, a) = cheby1(fs, f_low, f_high, ripple_db);
        Self { b, a, z1: 0.0, z2: 0.0 }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    #[inline]
    pub fn process_sample(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.z1;
        self.z1 = self.b[1] * x - self.a[1] * y + self.z2;
        self.z2 = self.b[2] * x - self.a[2] * y;
        y
    }

    pub fn process(&mut self, input: &[f64]) -> Vec<f64> {
        input.iter().map(|&x| self.process_sample(x)).collect()
    }
}

// ==========================================================
// Chebyshev Type I Bandpass Coefficient Design
// ==========================================================
/// Design a 2nd‑order Chebyshev Type I bandpass filter.
/// ripple_db = passband ripple in dB (e.g., 0.5)
fn cheby1(
    fs: f64,
    f_low: f64,
    f_high: f64,
    ripple_db: f64,
) -> ([f64; 3], [f64; 3]) {

    // Normalize frequencies
    let w0 = PI * (f_high + f_low) / fs;
    let bw = PI * (f_high - f_low) / fs;

    // Chebyshev ripple factor
    let eps = (10f64.powf(ripple_db / 10.0) - 1.0).sqrt();

    // Analog prototype pole (n = 2)
    let sinh_val = (eps).asinh() / 2.0;
    let cosh_val = (eps).asinh() / 2.0;

    let alpha = (bw / 2.0).sin() * sinh_val;
    let beta  = (bw / 2.0).sin() * cosh_val;

    // Raw coefficients
    let b0 =  alpha;
    let b1 =  0.0;
    let b2 = -alpha;

    let a0 =  1.0 + beta;
    let a1 = -2.0 * w0.cos();
    let a2 =  1.0 - beta;

    // Normalize
    let b = [b0 / a0, b1 / a0, b2 / a0];
    let a = [1.0, a1 / a0, a2 / a0];

    (b, a)
}


// ==========================================================
// TESTS
// ==========================================================
