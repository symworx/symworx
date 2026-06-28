// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

use crate::processing::interpolation::interp_linear;

/// Resampling structure.
/// Includes Linear, Cubic (not implemented), and Splined (not implemented) methods.
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
        Self {
            y,
            target_len: None,
        }
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
            ResampleMethod::Linear => self.linear_impl(),
            ResampleMethod::Cubic => todo!("Cubic resampling not implemented yet"),
            ResampleMethod::Spline => todo!("Spline resampling not implemented yet"),
        }
    }

    /// Internal linear interpolation method.
    fn linear_impl(&self) -> Vec<f64> {
        let old_len = self.y.len();
        let new_len = self.target_len.expect("target length not set");

        if old_len == 0 || new_len == 0 {
            return vec![];
        }

        let x: Vec<f64> = (0..old_len).map(|i| i as f64).collect();
        let x_new: Vec<f64> = (0..new_len)
            .map(|i| i as f64 * (old_len - 1) as f64 / (new_len - 1) as f64)
            .collect();

        interp_linear(&x, self.y, &x_new)
    }

    /// Quick implementation of the linear interpolation method.
    #[inline]
    pub fn linear(&self) -> Vec<f64> {
        self.method(ResampleMethod::Linear)
    }
}
