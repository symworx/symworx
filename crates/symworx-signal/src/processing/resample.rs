// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

use crate::processing::interpolation::{
    interp_linear,
    interp_spline,
};

type InterpFn = fn(&[f64], &[f64], &[f64]) -> Vec<f64>;

/// Resampling structure.
#[derive(Debug, Clone, Copy)]
pub enum ResampleMethod {
    /// Linear interpolation.
    Linear,
    /// Cubic interpolation.
    Cubic,
    /// Splined interpolation.
    Spline,
}

/// A resampler for 1D signals.
#[derive(Clone)]
pub struct Resample<'a> {
    /// Original signal to be resampled.
    y: &'a [f64],
    /// Desired output length.
    target_len: Option<usize>,
}

impl<'a> Resample<'a> {
    /// Create a new resampler method.
    #[inline]
    pub fn new(y: &'a [f64]) -> Self {
        Self { y, target_len: None }
    }

    /// Sets the target output length.
    #[allow(clippy::wrong_self_convention)]
    #[inline]
    pub fn to_len(&mut self, n: usize) {
        self.target_len = Some(n);
    }

    /// Sets the target length by applying a scaling factor.
    #[inline]
    pub fn scale(&mut self, factor: f64) {
        let old_len = self.y.len();
        let new_len = (old_len as f64 * factor).round().max(1.0) as usize;
        self.target_len = Some(new_len);
    }

    /// Sets the target length based on old and new sampling rates.
    #[allow(clippy::wrong_self_convention)]
    #[inline]
    pub fn to_rate(&mut self, old_fs: f64, new_fs: f64) {
        let old_len = self.y.len();
        let duration = old_len as f64 / old_fs;
        let new_len = (duration * new_fs).round().max(1.0) as usize;
        self.target_len = Some(new_len);
    }

    /// Resamples the signal using specified method.
    pub fn method(&self, method: ResampleMethod) -> Vec<f64> {
        match method {
            ResampleMethod::Linear => self.resample_with(interp_linear),
            ResampleMethod::Cubic | ResampleMethod::Spline => self.resample_with(interp_spline),
        }
    }

    fn resample_with(&self, interp: InterpFn) -> Vec<f64> {
        let old_len = self.y.len();
        let new_len = self.target_len.expect("target length not set");

        if old_len == 0 || new_len == 0 {
            return vec![];
        }

        let x: Vec<f64> = (0..old_len).map(|i| i as f64).collect();
        let x_new: Vec<f64> = if new_len == 1 {
            vec![0.0]
        } else {
            (0..new_len)
                .map(|i| i as f64 * (old_len - 1) as f64 / (new_len - 1) as f64)
                .collect()
        };

        interp(&x, self.y, &x_new)
    }

    /// Quick implementation of the linear interpolation method.
    #[inline]
    pub fn linear(&self) -> Vec<f64> {
        self.method(ResampleMethod::Linear)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spline_resample_length() {
        let y = [0.0, 1.0, 0.0, 1.0, 0.0];
        let mut r = Resample::new(&y);
        r.to_len(11);
        let out = r.method(ResampleMethod::Spline);
        assert_eq!(out.len(), 11);
        assert!((out[0] - 0.0).abs() < 1e-12);
        assert!((out[10] - 0.0).abs() < 1e-12);
        let cubic = r.method(ResampleMethod::Cubic);
        assert_eq!(cubic, out);
    }
}
