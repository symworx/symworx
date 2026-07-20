// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Linear Discriminant Analysis (LDA) classifier.
//!
//! Fits class means and a **pooled** covariance, then classifies with linear
//! discriminant scores (Bayes under shared Gaussian covariance).
//!
//! - **Fit** requires the `linalg` feature (matrix inverse of pooled Σ).
//! - **Predict** is a pure linear map — suitable to export for embedded use
//!   after training on a workstation (`score_k = x · coef_k + intercept_k`).
//!   See `docs/model_export.md` for C / mobile / web snippets.
//!
//! Labels are integer class indices. For multi-class, prediction is argmax of
//! discriminant scores (and softmax for probabilities).

use ndarray::{
    Array1,
    Array2,
};

#[cfg(feature = "linalg")]
use ndarray_linalg::Inverse;

/// Fitted LDA model (linear scores per class).
#[derive(Debug, Clone)]
pub struct LdaModel {
    /// Class labels (sorted unique).
    pub classes: Vec<usize>,
    /// Class priors π_k.
    pub priors: Array1<f64>,
    /// Class means (n_classes × n_features).
    pub means: Array2<f64>,
    /// Linear coefficients: `coef[[k, j]]` so score_k = x · coef_k + intercept_k.
    pub coef: Array2<f64>,
    /// Intercepts per class.
    pub intercept: Array1<f64>,
}

impl LdaModel {
    /// Number of classes.
    pub fn n_classes(&self) -> usize {
        self.classes.len()
    }

    /// Number of features.
    pub fn n_features(&self) -> usize {
        self.coef.ncols()
    }

    /// Discriminant scores (n_samples × n_classes).
    pub fn decision_function(&self, x: &Array2<f64>) -> Array2<f64> {
        assert_eq!(x.ncols(), self.n_features());
        // scores = X @ coef.T + intercept
        let mut scores = x.dot(&self.coef.t());
        for i in 0..scores.nrows() {
            for k in 0..self.n_classes() {
                scores[[i, k]] += self.intercept[k];
            }
        }
        scores
    }

    /// Softmax probabilities over discriminant scores.
    pub fn predict_proba(&self, x: &Array2<f64>) -> Array2<f64> {
        let scores = self.decision_function(x);
        let n = scores.nrows();
        let k = scores.ncols();
        let mut proba = Array2::<f64>::zeros((n, k));
        for i in 0..n {
            let mut m = f64::NEG_INFINITY;
            for c in 0..k {
                m = m.max(scores[[i, c]]);
            }
            let mut sum = 0.0;
            for c in 0..k {
                let e = (scores[[i, c]] - m).exp();
                proba[[i, c]] = e;
                sum += e;
            }
            for c in 0..k {
                proba[[i, c]] /= sum;
            }
        }
        proba
    }

    /// Predicted class labels.
    pub fn predict(&self, x: &Array2<f64>) -> Vec<usize> {
        let scores = self.decision_function(x);
        let mut out = Vec::with_capacity(x.nrows());
        for i in 0..x.nrows() {
            let mut best = 0usize;
            let mut best_v = f64::NEG_INFINITY;
            for c in 0..self.n_classes() {
                if scores[[i, c]] > best_v {
                    best_v = scores[[i, c]];
                    best = c;
                }
            }
            out.push(self.classes[best]);
        }
        out
    }
}

/// Fit LDA. Requires `linalg`.
///
/// Uses unbiased pooled covariance denominator `n − K` when `n > K`, else `n`.
#[cfg(feature = "linalg")]
pub fn lda(x: &Array2<f64>, y: &[usize]) -> LdaModel {
    assert_eq!(x.nrows(), y.len());
    assert!(!y.is_empty());
    assert!(x.ncols() > 0);

    let mut classes: Vec<usize> = y.to_vec();
    classes.sort_unstable();
    classes.dedup();
    let k = classes.len();
    let n = x.nrows();
    let p = x.ncols();
    assert!(k >= 2, "LDA needs at least 2 classes");
    assert!(
        n > k,
        "LDA needs n > n_classes for a non-degenerate pooled covariance"
    );

    let mut priors = Array1::<f64>::zeros(k);
    let mut means = Array2::<f64>::zeros((k, p));
    let mut counts = vec![0usize; k];

    for (ci, &cls) in classes.iter().enumerate() {
        let mut sum = Array1::<f64>::zeros(p);
        let mut count = 0usize;
        for i in 0..n {
            if y[i] == cls {
                count += 1;
                for j in 0..p {
                    sum[j] += x[[i, j]];
                }
            }
        }
        assert!(count > 0);
        counts[ci] = count;
        priors[ci] = count as f64 / n as f64;
        for j in 0..p {
            means[[ci, j]] = sum[j] / count as f64;
        }
    }

    // Pooled covariance
    let mut cov = Array2::<f64>::zeros((p, p));
    for i in 0..n {
        let ci = classes.iter().position(|&c| c == y[i]).unwrap();
        for a in 0..p {
            let da = x[[i, a]] - means[[ci, a]];
            for b in 0..p {
                let db = x[[i, b]] - means[[ci, b]];
                cov[[a, b]] += da * db;
            }
        }
    }
    let denom = (n - k) as f64;
    cov.mapv_inplace(|v| v / denom);

    // Ridge tiny diagonal for numerical stability
    for j in 0..p {
        cov[[j, j]] += 1e-9;
    }

    let cov_inv = cov
        .inv()
        .expect("LDA: pooled covariance inverse failed (singular features?)");

    // coef_k = Σ^{-1} μ_k
    // intercept_k = -0.5 μ_kᵀ Σ^{-1} μ_k + log π_k
    let mut coef = Array2::<f64>::zeros((k, p));
    let mut intercept = Array1::<f64>::zeros(k);
    for ci in 0..k {
        let mu = means.row(ci);
        let w = cov_inv.dot(&mu);
        for j in 0..p {
            coef[[ci, j]] = w[j];
        }
        intercept[ci] = -0.5 * mu.dot(&w) + priors[ci].ln();
    }

    LdaModel {
        classes,
        priors,
        means,
        coef,
        intercept,
    }
}

/// Stub when `linalg` is disabled.
#[cfg(not(feature = "linalg"))]
pub fn lda(_x: &Array2<f64>, _y: &[usize]) -> LdaModel {
    panic!(
        "symworx_stats::lda requires the `linalg` feature \
         (ndarray-linalg + LAPACK). Enable features = [\"linalg\"] on symworx-stats."
    );
}

#[cfg(all(test, feature = "linalg"))]
mod tests {
    use super::*;
    use ndarray::array;

    #[test]
    fn lda_two_class_separable() {
        let x = array![
            [0.0, 0.0],
            [0.1, 0.1],
            [0.0, 0.2],
            [0.2, 0.0],
            [5.0, 5.0],
            [5.1, 4.9],
            [4.9, 5.1],
            [5.0, 4.8],
        ];
        let y = vec![0, 0, 0, 0, 1, 1, 1, 1];
        let model = lda(&x, &y);
        let pred = model.predict(&x);
        assert_eq!(pred, y);
        let proba = model.predict_proba(&x);
        assert!(proba[[0, 0]] > 0.5);
        assert!(proba[[7, 1]] > 0.5);
    }
}
