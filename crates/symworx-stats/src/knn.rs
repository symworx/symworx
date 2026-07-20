// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! k-nearest neighbors classifier (multiclass).
//!
//! Lazy learner: stores the training set and classifies by majority vote among
//! the `k` nearest training points (optional distance-weighted votes).
//!
//! ## Platform note
//!
//! Inference needs the full training table in memory — fine for workstation /
//! teaching demos, **not** a default for tiny embedded targets. Prefer logistic,
//! LDA, or rules when shipping coefficients alone.
//!
//! Pure Rust; reuses [`crate::distance`].

use ndarray::Array2;

use crate::distance::{
    cosine_distance,
    euclidean,
    manhattan,
};

/// Distance metric for k-NN.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KnnMetric {
    /// Euclidean (L2). Default.
    #[default]
    Euclidean,
    /// Manhattan (L1).
    Manhattan,
    /// Cosine distance (`1 − cosine similarity`).
    Cosine,
}

impl KnnMetric {
    fn dist(self, a: &[f64], b: &[f64]) -> f64 {
        match self {
            KnnMetric::Euclidean => euclidean(a, b),
            KnnMetric::Manhattan => manhattan(a, b),
            KnnMetric::Cosine => cosine_distance(a, b),
        }
    }
}

/// Options for [`KnnClassifier`].
#[derive(Debug, Clone)]
pub struct KnnConfig {
    /// Number of neighbors (`k ≥ 1`).
    pub k: usize,
    /// Distance function.
    pub metric: KnnMetric,
    /// If `true`, weight votes by `1 / (distance + ε)` instead of uniform counts.
    pub weighted: bool,
}

impl Default for KnnConfig {
    fn default() -> Self {
        Self {
            k: 3,
            metric: KnnMetric::Euclidean,
            weighted: false,
        }
    }
}

/// Fitted (stored) multiclass k-NN model.
#[derive(Debug, Clone)]
pub struct KnnClassifier {
    /// Training design matrix (n_train × n_features).
    pub x_train: Array2<f64>,
    /// Training labels (length n_train), integer class indices.
    pub y_train: Vec<usize>,
    /// Sorted unique class labels seen at fit.
    pub classes: Vec<usize>,
    /// Neighbor / metric settings.
    pub config: KnnConfig,
}

impl KnnClassifier {
    /// Fit by storing training data (lazy).
    ///
    /// # Panics
    /// Panics if `x` is empty, row counts differ, `k == 0`, or `k > n_train`.
    pub fn fit(x: &Array2<f64>, y: &[usize], config: KnnConfig) -> Self {
        assert_eq!(x.nrows(), y.len(), "X and y length mismatch");
        assert!(x.nrows() > 0, "need at least one training sample");
        assert!(x.ncols() > 0, "need at least one feature");
        assert!(config.k >= 1, "k must be ≥ 1");
        assert!(
            config.k <= x.nrows(),
            "k ({}) cannot exceed n_train ({})",
            config.k,
            x.nrows()
        );

        let mut classes = y.to_vec();
        classes.sort_unstable();
        classes.dedup();

        Self {
            x_train: x.clone(),
            y_train: y.to_vec(),
            classes,
            config,
        }
    }

    /// Convenience fit with default config (override `k` only).
    pub fn fit_k(x: &Array2<f64>, y: &[usize], k: usize) -> Self {
        Self::fit(
            x,
            y,
            KnnConfig {
                k,
                ..Default::default()
            },
        )
    }

    /// Number of training samples stored.
    pub fn n_train(&self) -> usize {
        self.y_train.len()
    }

    /// Number of features.
    pub fn n_features(&self) -> usize {
        self.x_train.ncols()
    }

    /// Number of classes.
    pub fn n_classes(&self) -> usize {
        self.classes.len()
    }

    /// Predict class labels for each row of `x`.
    pub fn predict(&self, x: &Array2<f64>) -> Vec<usize> {
        assert_eq!(x.ncols(), self.n_features());
        (0..x.nrows())
            .map(|i| {
                let row: Vec<f64> = x.row(i).to_vec();
                self.predict_one(&row)
            })
            .collect()
    }

    /// Soft vote fractions over `classes` (n_samples × n_classes).
    ///
    /// Columns align with [`KnnClassifier::classes`]. Uniform or distance-weighted
    /// according to config.
    pub fn predict_proba(&self, x: &Array2<f64>) -> Array2<f64> {
        assert_eq!(x.ncols(), self.n_features());
        let mut proba = Array2::<f64>::zeros((x.nrows(), self.n_classes()));
        for i in 0..x.nrows() {
            let row: Vec<f64> = x.row(i).to_vec();
            let votes = self.vote_weights(&row);
            let sum: f64 = votes.iter().sum();
            if sum > 0.0 {
                for c in 0..self.n_classes() {
                    proba[[i, c]] = votes[c] / sum;
                }
            } else if !self.classes.is_empty() {
                // Fallback uniform
                let u = 1.0 / self.n_classes() as f64;
                for c in 0..self.n_classes() {
                    proba[[i, c]] = u;
                }
            }
        }
        proba
    }

    fn predict_one(&self, row: &[f64]) -> usize {
        let votes = self.vote_weights(row);
        let mut best_c = 0usize;
        let mut best_v = f64::NEG_INFINITY;
        for (c, &v) in votes.iter().enumerate() {
            // Prefer higher vote; ties → lower class index (deterministic)
            if v > best_v || (v == best_v && c < best_c) {
                best_v = v;
                best_c = c;
            }
        }
        self.classes[best_c]
    }

    /// Vote weight per class index into `self.classes`.
    fn vote_weights(&self, row: &[f64]) -> Vec<f64> {
        let n = self.n_train();
        let k = self.config.k.min(n);

        // (distance, train_index)
        let mut dists: Vec<(f64, usize)> = (0..n)
            .map(|i| {
                let ti: Vec<f64> = self.x_train.row(i).to_vec();
                (self.config.metric.dist(row, &ti), i)
            })
            .collect();
        // Partial sort: k smallest distances
        dists.select_nth_unstable_by(k - 1, |a, b| a.0.partial_cmp(&b.0).unwrap());
        dists[..k].sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let mut votes = vec![0.0; self.n_classes()];
        for &(d, idx) in dists.iter().take(k) {
            let label = self.y_train[idx];
            let Some(ci) = self.classes.iter().position(|&c| c == label) else {
                continue;
            };
            let w = if self.config.weighted {
                1.0 / (d + 1e-12)
            } else {
                1.0
            };
            votes[ci] += w;
        }
        votes
    }
}

/// Fit multiclass k-NN with the given config.
pub fn knn_classify(x: &Array2<f64>, y: &[usize], config: &KnnConfig) -> KnnClassifier {
    KnnClassifier::fit(x, y, config.clone())
}

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    #[test]
    fn two_class_blobs() {
        let x = array![
            [0.0, 0.0],
            [0.1, 0.0],
            [0.0, 0.1],
            [5.0, 5.0],
            [5.1, 5.0],
            [5.0, 5.1],
        ];
        let y = vec![0, 0, 0, 1, 1, 1];
        let clf = KnnClassifier::fit_k(&x, &y, 3);
        let pred = clf.predict(&x);
        assert_eq!(pred, y);
    }

    #[test]
    fn three_class() {
        let x = array![
            [0.0, 0.0],
            [0.1, 0.0],
            [3.0, 0.0],
            [3.1, 0.0],
            [0.0, 3.0],
            [0.0, 3.1],
        ];
        let y = vec![0, 0, 1, 1, 2, 2];
        let clf = knn_classify(
            &x,
            &y,
            &KnnConfig {
                k: 1,
                ..Default::default()
            },
        );
        assert_eq!(clf.predict(&x), y);
        assert_eq!(clf.n_classes(), 3);
    }

    #[test]
    fn proba_sums_to_one() {
        let x = array![[0.0, 0.0], [0.1, 0.0], [5.0, 5.0], [5.1, 5.0]];
        let y = vec![0, 0, 1, 1];
        let clf = KnnClassifier::fit_k(&x, &y, 2);
        let p = clf.predict_proba(&x);
        for i in 0..p.nrows() {
            let s: f64 = p.row(i).sum();
            assert!((s - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn weighted_runs() {
        let x = array![[0.0], [1.0], [10.0], [11.0]];
        let y = vec![0, 0, 1, 1];
        let clf = knn_classify(
            &x,
            &y,
            &KnnConfig {
                k: 2,
                weighted: true,
                metric: KnnMetric::Manhattan,
            },
        );
        assert_eq!(clf.predict(&array![[0.2]])[0], 0);
        assert_eq!(clf.predict(&array![[10.5]])[0], 1);
    }
}
