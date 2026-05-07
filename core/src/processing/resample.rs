// core/src/processing/resample.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use crate::processing::interpolation::interp_linear;

// ==========================================================
// RESAMPLING METHOD
// ==========================================================
#[derive(Debug, Clone, Copy)]
pub enum ResampleMethod {
    Linear,
    Cubic,
    Spline,
}

#[derive(Clone)]
pub struct Resample<'a> {
    y: &'a [f64],
    target_len: Option<usize>,
}

impl<'a> Resample<'a> {
    #[inline]
    pub fn new(y: &'a [f64]) -> Self {
        Self {
            y,
            target_len: None,
        }
    }

    #[allow(clippy::wrong_self_convention)]
    #[inline]
    pub fn to_len(&mut self, n: usize) {
        self.target_len = Some(n);
    }

    #[inline]
    pub fn scale(&mut self, factor: f64) {
        let old_len = self.y.len();
        let new_len = (old_len as f64 * factor).round().max(1.0) as usize;
        self.target_len = Some(new_len);
    }

    #[allow(clippy::wrong_self_convention)]
    #[inline]
    pub fn to_rate(&mut self, old_fs: f64, new_fs: f64) {
        let old_len = self.y.len();
        let duration = old_len as f64 / old_fs;
        let new_len = (duration * new_fs).round().max(1.0) as usize;
        self.target_len = Some(new_len);
    }

    pub fn method(&self, method: ResampleMethod) -> Vec<f64> {
        match method {
            ResampleMethod::Linear => self.linear_impl(),
            ResampleMethod::Cubic  => todo!("Cubic resampling not implemented yet"),
            ResampleMethod::Spline => todo!("Spline resampling not implemented yet"),
        }
    }

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

    #[inline]
    pub fn linear(&self) -> Vec<f64> {
        self.method(ResampleMethod::Linear)
    }
}
