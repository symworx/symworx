// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Scalar smoothers: centered moving average, causal EMA, and one biquad section.
//!
//! The bandpass design matches `symworx-signal`'s 2nd-order Butterworth section
//! (bilinear transform, direct form II transposed). Cascading `stages` sections
//! is the same pattern `symworx-biosym` uses before peak detection.

use alloc::vec::Vec;

use crate::NpuError;

/// Centered moving average.
///
/// `kernel` of 0 or 1 copies `data`. At each index `i` the window is
/// `i.saturating_sub(kernel / 2) .. min(i + kernel / 2 + 1, len)`.
pub fn moving_average(data: &[f64], kernel: usize) -> Vec<f64> {
    if kernel <= 1 || data.is_empty() {
        return data.to_vec();
    }
    let half = kernel / 2;
    let mut out = Vec::with_capacity(data.len());
    for i in 0..data.len() {
        let start = i.saturating_sub(half);
        let end = (i + half + 1).min(data.len());
        let sum: f64 = data[start..end].iter().sum();
        out.push(sum / (end - start) as f64);
    }
    out
}

/// Causal exponential moving average. The first finite sample seeds the state.
#[derive(Debug, Clone)]
pub struct Ema {
    alpha: f64,
    state: Option<f64>,
}

impl Ema {
    /// `alpha` is the weight on the new sample. `1` copies the input.
    pub fn new(alpha: f64) -> Result<Self, NpuError> {
        if !alpha.is_finite() || alpha <= 0.0 || alpha > 1.0 {
            return Err(NpuError::InvalidInput("ema alpha must be in (0, 1]"));
        }
        Ok(Self { alpha, state: None })
    }

    /// Forget the seed so the next sample starts a new average.
    pub fn reset(&mut self) {
        self.state = None;
    }

    /// Push one sample. A non-finite sample is returned unchanged and does not move the state.
    pub fn push(&mut self, x: f64) -> f64 {
        if !x.is_finite() {
            return x;
        }
        let y = match self.state {
            Some(prev) => self.alpha * x + (1.0 - self.alpha) * prev,
            None => x,
        };
        self.state = Some(y);
        y
    }

    /// Filter a whole buffer. State starts empty.
    pub fn apply(alpha: f64, data: &[f64]) -> Result<Vec<f64>, NpuError> {
        let mut ema = Self::new(alpha)?;
        Ok(data.iter().map(|&x| ema.push(x)).collect())
    }
}

/// One biquad section, direct form II transposed, with `a0` normalized to 1.
#[derive(Debug, Clone)]
pub struct Biquad {
    b: [f64; 3],
    a1: f64,
    a2: f64,
    z1: f64,
    z2: f64,
}

impl Biquad {
    /// Build from transfer-function coefficients. `a[0]` must be finite and non-zero.
    pub fn from_coeffs(b: [f64; 3], a: [f64; 3]) -> Result<Self, NpuError> {
        if !a[0].is_finite() || a[0] == 0.0 || !coeffs_finite(&b) || !a[1].is_finite() || !a[2].is_finite() {
            return Err(NpuError::InvalidInput(
                "biquad coefficients must be finite and a0 non-zero",
            ));
        }
        let inv = 1.0 / a[0];
        Ok(Self {
            b: [b[0] * inv, b[1] * inv, b[2] * inv],
            a1: a[1] * inv,
            a2: a[2] * inv,
            z1: 0.0,
            z2: 0.0,
        })
    }

    /// 2nd-order Butterworth bandpass. `q` is the quality factor (0.707 is the usual section).
    pub fn bandpass(fs: f64, f_low: f64, f_high: f64, q: f64) -> Result<Self, NpuError> {
        let (b, a) = design_bandpass(fs, f_low, f_high, q)?;
        Self::from_coeffs(b, a)
    }

    /// Clear the two delays.
    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    /// One sample. A non-finite input is returned unchanged and does not move the delays.
    #[inline]
    pub fn process_sample(&mut self, x: f64) -> f64 {
        if !x.is_finite() {
            return x;
        }
        let y = self.b[0] * x + self.z1;
        self.z1 = self.b[1] * x - self.a1 * y + self.z2;
        self.z2 = self.b[2] * x - self.a2 * y;
        y
    }

    /// Filter a buffer from the current delay state.
    pub fn process(&mut self, input: &[f64]) -> Vec<f64> {
        input.iter().map(|&x| self.process_sample(x)).collect()
    }
}

/// Cascaded bandpass. `stages` of 0 is treated as one section.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandpassSpec {
    /// Sample rate (Hz).
    pub fs: f64,
    /// Low cutoff (Hz).
    pub low_hz: f64,
    /// High cutoff (Hz).
    pub high_hz: f64,
    /// Section quality factor.
    pub q: f64,
    /// Number of cascaded 2nd-order sections.
    pub stages: u8,
}

impl BandpassSpec {
    /// Run `stages` identical sections in series. Each section starts from zero delays.
    pub fn apply(&self, signal: &[f64]) -> Result<Vec<f64>, NpuError> {
        let stages = self.stages.max(1);
        let mut out = signal.to_vec();
        for _ in 0..stages {
            let mut section = Biquad::bandpass(self.fs, self.low_hz, self.high_hz, self.q)?;
            out = section.process(&out);
        }
        Ok(out)
    }
}

fn coeffs_finite(b: &[f64; 3]) -> bool {
    b.iter().all(|c| c.is_finite())
}

fn design_bandpass(fs: f64, f_low: f64, f_high: f64, q: f64) -> Result<([f64; 3], [f64; 3]), NpuError> {
    if !fs.is_finite() || fs <= 0.0 {
        return Err(NpuError::InvalidInput("sample rate must be finite and positive"));
    }
    if !f_low.is_finite() || !f_high.is_finite() || !q.is_finite() {
        return Err(NpuError::InvalidInput("bandpass cutoffs and q must be finite"));
    }
    if f_low <= 0.0 || f_high <= f_low {
        return Err(NpuError::InvalidInput("bandpass requires 0 < low < high"));
    }
    if f_high >= fs / 2.0 {
        return Err(NpuError::InvalidInput("bandpass high cutoff must be below Nyquist"));
    }
    if q <= 0.0 {
        return Err(NpuError::InvalidInput("bandpass q must be positive"));
    }

    let w0 = core::f64::consts::PI * (f_high + f_low) / fs;
    let bw = core::f64::consts::PI * (f_high - f_low) / fs;
    let alpha = (bw / 2.0).sin() / (2.0 * q);

    let b0 = alpha;
    let b1 = 0.0;
    let b2 = -alpha;
    let a0 = 1.0 + alpha;
    let a1 = -2.0 * w0.cos();
    let a2 = 1.0 - alpha;

    Ok(([b0 / a0, b1 / a0, b2 / a0], [1.0, a1 / a0, a2 / a0]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_average_shrinks_the_window_at_the_ends() {
        let got = moving_average(&[0.0, 0.0, 3.0, 0.0, 0.0], 3);
        assert_eq!(got, vec![0.0, 1.0, 1.0, 1.0, 0.0]);
        assert_eq!(moving_average(&[1.0, 2.0], 1), vec![1.0, 2.0]);
        assert!(moving_average(&[], 5).is_empty());
    }

    #[test]
    fn ema_seeds_on_the_first_sample_and_rejects_a_bad_alpha() {
        let mut ema = Ema::new(0.5).unwrap();
        assert_eq!(ema.push(0.0), 0.0);
        assert_eq!(ema.push(10.0), 5.0);
        assert!(ema.push(f64::NAN).is_nan());
        assert_eq!(ema.push(10.0), 7.5);
        assert!(Ema::new(0.0).is_err());
        assert!(Ema::new(1.5).is_err());
    }

    #[test]
    fn bandpass_matches_the_host_bilinear_section() {
        let filter = Biquad::bandpass(100.0, 0.2, 6.0, 0.707).unwrap();
        assert!((filter.b[0] - 0.06045275789514596).abs() < 1e-12);
        assert_eq!(filter.b[1], 0.0);
        assert!((filter.b[0] + filter.b[2]).abs() < 1e-12);
        assert!((filter.a1 - -1.8435617798382176).abs() < 1e-12);
        assert!((filter.a2 - 0.8790944842097081).abs() < 1e-12);
    }

    #[test]
    fn bandpass_rejects_a_cutoff_at_or_above_nyquist() {
        let err = Biquad::bandpass(10.0, 0.2, 6.0, 0.707).unwrap_err();
        assert!(matches!(
            err,
            NpuError::InvalidInput("bandpass high cutoff must be below Nyquist")
        ));
    }

    #[test]
    fn cascaded_bandpass_kills_a_constant() {
        let spec = BandpassSpec {
            fs: 100.0,
            low_hz: 0.2,
            high_hz: 6.0,
            q: 0.707,
            stages: 2,
        };
        let y = spec.apply(&[1.0; 400]).unwrap();
        let tail = &y[300..];
        let mean = tail.iter().sum::<f64>() / tail.len() as f64;
        assert!(mean.abs() < 1e-3, "tail mean {mean}");
    }
}
