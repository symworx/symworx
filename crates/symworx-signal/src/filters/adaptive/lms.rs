// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Adaptive filters: Least Mean Squares (LMS) and Normalized LMS (NLMS).
//!
//! Adaptive FIR filters useful for noise cancellation, system identification,
//! and echo cancellation.

/// Standard Least Mean Squares (LMS) adaptive filter.
#[derive(Debug, Clone)]
pub struct LmsFilter {
    /// Weights used in LMS
    weights: Vec<f64>,
    /// Step size
    mu: f64,
    /// Length
    length: usize,
}

/// Normalized Least Mean Squares (NLMS) adaptive filter.
///
/// More stable than standard LMS because the step size is normalized
/// by the input signal power.
#[derive(Debug, Clone)]
pub struct NlmsFilter {
    weights: Vec<f64>,
    mu: f64, // normalized step size (typically 0.1 - 1.0)
    length: usize,
    epsilon: f64, // small constant to avoid division by zero
}

impl LmsFilter {
    /// Initiate a new LMS Filter
    pub fn new(length: usize, mu: f64) -> Self {
        assert!(length > 0);
        assert!(mu > 0.0);
        Self {
            weights: vec![0.0; length],
            mu,
            length,
        }
    }

    /// Perform one adaptation step using the LMS algorithm.
    pub fn adapt(&mut self, input: f64, desired: f64) -> f64 {
        let input_vec: Vec<f64> = std::iter::once(input)
            .chain(self.weights.iter().copied().take(self.length - 1))
            .collect();

        let output: f64 = self.weights.iter().zip(&input_vec).map(|(&w, &x)| w * x).sum();
        let error = desired - output;

        for (w, &x) in self.weights.iter_mut().zip(&input_vec) {
            *w += self.mu * error * x;
        }

        error
    }

    /// Process a batch of samples with LMS adaptation.
    pub fn process(&mut self, inputs: &[f64], desired: &[f64]) -> Vec<f64> {
        assert_eq!(inputs.len(), desired.len());
        inputs.iter().zip(desired).map(|(&x, &d)| self.adapt(x, d)).collect()
    }

    /// Returns a copy of the current filter weights.
    pub fn weights(&self) -> Vec<f64> {
        self.weights.clone()
    }

    /// Reset all filter weights to zero.
    pub fn reset(&mut self) {
        self.weights.fill(0.0);
    }
}

impl NlmsFilter {
    /// Initiate a new NLMS filter
    pub fn new(length: usize, mu: f64, epsilon: f64) -> Self {
        assert!(length > 0);
        assert!(mu > 0.0);
        Self {
            weights: vec![0.0; length],
            mu,
            length,
            epsilon: epsilon.max(1e-6),
        }
    }

    /// Perform one adaptation step using the NLMS algorithm.
    pub fn adapt(&mut self, input: f64, desired: f64) -> f64 {
        let input_vec: Vec<f64> = std::iter::once(input)
            .chain(self.weights.iter().copied().take(self.length - 1))
            .collect();

        let output: f64 = self.weights.iter().zip(&input_vec).map(|(&w, &x)| w * x).sum();
        let error = desired - output;

        // Compute input power
        let power: f64 = input_vec.iter().map(|&x| x * x).sum();
        let norm = power + self.epsilon;

        for (w, &x) in self.weights.iter_mut().zip(&input_vec) {
            *w += (self.mu / norm) * error * x;
        }

        error
    }

    /// Process a batch of samples with NLMS adaptation.
    pub fn process(&mut self, inputs: &[f64], desired: &[f64]) -> Vec<f64> {
        assert_eq!(inputs.len(), desired.len());
        inputs.iter().zip(desired).map(|(&x, &d)| self.adapt(x, d)).collect()
    }

    /// Returns a copy of the current filter weights.
    pub fn weights(&self) -> Vec<f64> {
        self.weights.clone()
    }

    /// Reset all filter weights to zero.
    pub fn reset(&mut self) {
        self.weights.fill(0.0);
    }
}

// ——— Tests ———

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lms_and_nlms_creation() {
        let _lms = LmsFilter::new(8, 0.05);
        let _nlms = NlmsFilter::new(8, 0.5, 1e-6);
    }

    #[test]
    fn test_lms_adaptation() {
        let mut filter = LmsFilter::new(4, 0.1);
        let inputs = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let desired: Vec<f64> = inputs.iter().map(|&x| 2.0 * x).collect();

        let errors = filter.process(&inputs, &desired);
        assert!(errors.last().unwrap().abs() < 0.8);
    }

    #[test]
    fn test_nlms_adaptation() {
        let mut filter = NlmsFilter::new(4, 0.8, 1e-6);
        let inputs = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let desired: Vec<f64> = inputs.iter().map(|&x| 2.0 * x).collect();

        let errors = filter.process(&inputs, &desired);
        assert!(errors.last().unwrap().abs() < 0.5);
    }
}
