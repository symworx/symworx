// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Recursive Least Squares (RLS) adaptive filter.
//!
//! RLS offers faster convergence than LMS/NLMS at the cost of higher
//! computational complexity (O(n²) per update).

/// Recursive Least Squares (RLS) adaptive filter.
#[derive(Debug, Clone)]
pub struct RlsFilter {
    /// Filter weights (coefficients)
    weights: Vec<f64>,
    /// Inverse correlation matrix (P)
    p: Vec<Vec<f64>>,
    /// Forgetting factor (0 < lambda <= 1). Closer to 1 = longer memory.
    lambda: f64,
    /// Regularization parameter (initial P = delta * I)
    delta: f64,
    length: usize,
}

impl RlsFilter {
    /// Creates a new RLS adaptive filter.
    ///
    /// # Arguments
    /// * `length` — Filter length (number of taps)
    /// * `lambda` — Forgetting factor (typically 0.95 – 0.999)
    /// * `delta`  — Initial regularization (typically 0.1 – 1000.0)
    pub fn new(length: usize, lambda: f64, delta: f64) -> Self {
        assert!(length > 0);
        assert!((0.0..=1.0).contains(&lambda));
        assert!(delta > 0.0);

        let mut p = vec![vec![0.0; length]; length];
        for i in 0..length {
            p[i][i] = delta;
        }

        Self {
            weights: vec![0.0; length],
            p,
            lambda,
            delta,
            length,
        }
    }

    /// Performs one RLS adaptation step.
    pub fn adapt(&mut self, input: f64, desired: f64) -> f64 {
        let input_vec: Vec<f64> = std::iter::once(input)
            .chain(
                self.weights
                    .iter()
                    .copied()
                    .take(self.length.saturating_sub(1)),
            )
            .collect();

        // Compute output
        let output: f64 = self
            .weights
            .iter()
            .zip(&input_vec)
            .map(|(&w, &x)| w * x)
            .sum();

        let error = desired - output;

        // Compute gain vector k
        let mut pi = vec![0.0; self.length];
        for i in 0..self.length {
            for j in 0..self.length {
                pi[i] += self.p[i][j] * input_vec[j];
            }
        }

        let lambda_pi_input =
            self.lambda + pi.iter().zip(&input_vec).map(|(&p, &x)| p * x).sum::<f64>();
        let k: Vec<f64> = pi.iter().map(|&val| val / lambda_pi_input).collect();

        // Update weights
        for (w, &ki) in self.weights.iter_mut().zip(&k) {
            *w += ki * error;
        }

        // Update inverse correlation matrix P
        for i in 0..self.length {
            for j in 0..self.length {
                let outer = k[i] * pi[j];
                self.p[i][j] = (self.p[i][j] - outer) / self.lambda;
            }
        }

        error
    }

    /// Process multiple samples with online adaptation.
    pub fn process(&mut self, inputs: &[f64], desired: &[f64]) -> Vec<f64> {
        assert_eq!(inputs.len(), desired.len());
        inputs
            .iter()
            .zip(desired.iter())
            .map(|(&x, &d)| self.adapt(x, d))
            .collect()
    }

    /// Clone the weights for analysis.
    pub fn weights(&self) -> Vec<f64> {
        self.weights.clone()
    }

    /// Reset weights used in analysis.
    pub fn reset(&mut self) {
        self.weights.fill(0.0);
        for i in 0..self.length {
            for j in 0..self.length {
                self.p[i][j] = if i == j { self.delta } else { 0.0 };
            }
        }
    }
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rls_creation() {
        let rls = RlsFilter::new(4, 0.98, 100.0);
        assert_eq!(rls.weights().len(), 4);
    }

    #[test]
    fn test_rls_adaptation() {
        let mut rls = RlsFilter::new(4, 0.97, 50.0);

        let inputs = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let desired: Vec<f64> = inputs.iter().map(|&x| x * 2.0 + 0.5).collect();

        let errors = rls.process(&inputs, &desired);

        // Error should generally decrease
        assert!(errors.last().unwrap().abs() < 2.0);
    }
}
