// symworx/crates/symworx-signal/src/filters/linear/bandpass.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

// ==========================================================
// Butterworth Bandpass Filter
// ==========================================================
/// Butterworth bandpass filter 
pub struct BandpassFilter {
    a: [f64; 3],
    b: [f64; 3],
    z1: f64,
    z2: f64, 
}

impl BandpassFilter {
    // Create a new bandpass filter
    pub fn new(fs: f64, f_low: f64, f_high: f64, q: f64) -> Self {
        let (b, a) = bandpass(fs, f_low, f_high, q);
        Self { b, a, z1: 0.0, z2: 0.0 }
    }

    // Resets internal state
    #[inline]
    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    // Process sample
    #[inline]
    pub fn process_sample(&mut self, x: f64) -> f64 {
        // Direct Form II Transposed
        let y = self.b[0] * x + self.z1;
        self.z1 = self.b[1] * x - self.a[1] * y + self.z2;
        self.z2 = self.b[2] * x - self.a[2] * y;
        y
    }

    // Process full signal
    pub fn process(&mut self, input: &[f64]) -> Vec<f64> {
        input.iter().map(|&x| self.process_sample(x)).collect()
    }
}

// ==========================================================
// Butterworth Bandpass Filter (second order)
// ==========================================================
/// Butterworth bandpass filter (second order) using the bilinear transform.
/// 
/// Returns (b, a) where:
///   y[n] = b0*x[n] + b1*x[n-1] + b2*x[n-2] - a1*y[n-1] - a2*y[n-2]
fn bandpass(fs: f64, f_low: f64, f_high: f64, q: f64) -> ([f64; 3], [f64; 3]) {
    // Normalize frequencies
    let w0 = std::f64::consts::PI * (f_high + f_low) / fs;
    let bw = std::f64::consts::PI * (f_high - f_low) / fs;

    // Bandwidth shaping
    let alpha = (bw / 2.0).sin() / (2.0 * q);

    // Raw coefficients
    let b0 =  alpha;
    let b1 =  0.0;
    let b2 = -alpha;

    let a0 =  1.0 + alpha;
    let a1 = -2.0 * w0.cos();
    let a2 =  1.0 - alpha;

    // Normalize
    let b = [b0 / a0, b1 / a0, b2 / a0];
    let a = [1.0, a1 / a0, a2 / a0];

    (b, a)
}


// ==========================================================
// TESTS
// ==========================================================
