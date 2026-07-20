// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Gaussian Naive Bayes classifier.
//!
//! Assumes features are independent Gaussians within each class. Fit stores
//! per-class priors, means, and variances — small enough to ship to embedded
//! targets for linear-cost inference (`O(n_features · n_classes)`).
//!
//! Pure Rust — no `linalg` / LAPACK. Labels are integer class indices `0..K`.

use ndarray::{
    Array1,
    Array2,
};

/// Fitted Gaussian Naive Bayes model.
#[derive(Debug, Clone)]
pub struct GaussianNb {
    /// Class indices present at fit time (sorted unique, usually `0..K`).
    pub classes: Vec<usize>,
    /// Log class priors `log π_k` (aligned with `classes`).
    pub log_priors: Array1<f64>,
    /// Class means: `means[[k, j]]` for class `classes[k]`, feature `j`.
    pub means: Array2<f64>,
    /// Class variances (with variance smoothing): same shape as `means`.
    pub vars: Array2<f64>,
}

impl GaussianNb {
    /// Number of classes.
    pub fn n_classes(&self) -> usize {
        self.classes.len()
    }

    /// Number of features.
    pub fn n_features(&self) -> usize {
        self.means.ncols()
    }

    /// Unnormalized log joint `log P(x, y=c)` for each class (rows = samples).
    pub fn predict_log_proba(&self, x: &Array2<f64>) -> Array2<f64> {
        assert_eq!(x.ncols(), self.n_features());
        let n = x.nrows();
        let k = self.n_classes();
        let mut out = Array2::<f64>::zeros((n, k));
        const LN_2PI: f64 = 1.837_877_066_409_345_3; // ln(2π)

        for i in 0..n {
            for c in 0..k {
                let mut log_lik = self.log_priors[c];
                for j in 0..self.n_features() {
                    let mean = self.means[[c, j]];
                    let var = self.vars[[c, j]];
                    let diff = x[[i, j]] - mean;
                    // log N(x; μ, σ²) = -0.5 ln(2πσ²) - (x-μ)²/(2σ²)
                    log_lik += -0.5 * (LN_2PI + var.ln()) - diff * diff / (2.0 * var);
                }
                out[[i, c]] = log_lik;
            }
        }
        out
    }

    /// Class probabilities (softmax of log joints) for each sample.
    pub fn predict_proba(&self, x: &Array2<f64>) -> Array2<f64> {
        let log_p = self.predict_log_proba(x);
        let n = log_p.nrows();
        let k = log_p.ncols();
        let mut proba = Array2::<f64>::zeros((n, k));
        for i in 0..n {
            let mut max_lp = f64::NEG_INFINITY;
            for c in 0..k {
                max_lp = max_lp.max(log_p[[i, c]]);
            }
            let mut sum = 0.0;
            for c in 0..k {
                let e = (log_p[[i, c]] - max_lp).exp();
                proba[[i, c]] = e;
                sum += e;
            }
            for c in 0..k {
                proba[[i, c]] /= sum;
            }
        }
        proba
    }

    /// Predicted class labels (values from `classes`).
    pub fn predict(&self, x: &Array2<f64>) -> Vec<usize> {
        let log_p = self.predict_log_proba(x);
        let mut labels = Vec::with_capacity(x.nrows());
        for i in 0..x.nrows() {
            let mut best = 0usize;
            let mut best_v = f64::NEG_INFINITY;
            for c in 0..self.n_classes() {
                if log_p[[i, c]] > best_v {
                    best_v = log_p[[i, c]];
                    best = c;
                }
            }
            labels.push(self.classes[best]);
        }
        labels
    }
}

/// Options for [`gaussian_nb`].
#[derive(Debug, Clone)]
pub struct GaussianNbConfig {
    /// Added to empirical variance (`var_smoothing`); avoids zero variance.
    pub var_smoothing: f64,
}

impl Default for GaussianNbConfig {
    fn default() -> Self {
        Self {
            var_smoothing: 1e-9,
        }
    }
}

/// Fit Gaussian Naive Bayes on design matrix `x` and integer labels `y`.
///
/// # Panics
/// Panics if row counts differ, `x` is empty, or `y` is empty.
pub fn gaussian_nb(x: &Array2<f64>, y: &[usize], config: &GaussianNbConfig) -> GaussianNb {
    assert_eq!(x.nrows(), y.len(), "X and y length mismatch");
    assert!(!y.is_empty(), "need at least one sample");
    assert!(x.ncols() > 0, "need at least one feature");
    assert!(config.var_smoothing >= 0.0);

    let mut classes: Vec<usize> = y.to_vec();
    classes.sort_unstable();
    classes.dedup();
    let k = classes.len();
    let p = x.ncols();
    let n = x.nrows() as f64;

    // Global variance scale for smoothing (sklearn-style ε · max(var))
    let mut global_max_var: f64 = 0.0;
    for j in 0..p {
        let mean_j = x.column(j).mean().unwrap_or(0.0);
        let mut ss = 0.0;
        for i in 0..x.nrows() {
            let d = x[[i, j]] - mean_j;
            ss += d * d;
        }
        global_max_var = global_max_var.max(ss / n);
    }
    let eps = config.var_smoothing * global_max_var.max(1e-12);

    let mut log_priors = Array1::<f64>::zeros(k);
    let mut means = Array2::<f64>::zeros((k, p));
    let mut vars = Array2::<f64>::zeros((k, p));

    for (ci, &cls) in classes.iter().enumerate() {
        let mut count = 0usize;
        let mut sum = Array1::<f64>::zeros(p);
        for i in 0..x.nrows() {
            if y[i] == cls {
                count += 1;
                for j in 0..p {
                    sum[j] += x[[i, j]];
                }
            }
        }
        assert!(count > 0);
        let cn = count as f64;
        log_priors[ci] = (cn / n).ln();
        for j in 0..p {
            means[[ci, j]] = sum[j] / cn;
        }
        let mut ss = Array1::<f64>::zeros(p);
        for i in 0..x.nrows() {
            if y[i] == cls {
                for j in 0..p {
                    let d = x[[i, j]] - means[[ci, j]];
                    ss[j] += d * d;
                }
            }
        }
        for j in 0..p {
            // Population variance within class + smoothing
            vars[[ci, j]] = ss[j] / cn + eps;
        }
    }

    GaussianNb {
        classes,
        log_priors,
        means,
        vars,
    }
}

/// Fit with default config.
pub fn gaussian_nb_default(x: &Array2<f64>, y: &[usize]) -> GaussianNb {
    gaussian_nb(x, y, &GaussianNbConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    #[test]
    fn separates_two_blobs() {
        let x = array![
            [0.0, 0.0],
            [0.1, 0.1],
            [0.0, 0.2],
            [5.0, 5.0],
            [5.1, 4.9],
            [4.9, 5.1],
        ];
        let y = vec![0, 0, 0, 1, 1, 1];
        let model = gaussian_nb_default(&x, &y);
        let pred = model.predict(&x);
        assert_eq!(pred, y);
        let proba = model.predict_proba(&x);
        assert!(proba[[0, 0]] > 0.5);
        assert!(proba[[5, 1]] > 0.5);
    }

    #[test]
    fn three_classes() {
        let x = array![
            [0.0, 0.0],
            [0.1, 0.0],
            [3.0, 0.0],
            [3.1, 0.1],
            [0.0, 3.0],
            [0.1, 3.1],
        ];
        let y = vec![0, 0, 1, 1, 2, 2];
        let model = gaussian_nb_default(&x, &y);
        assert_eq!(model.n_classes(), 3);
        let pred = model.predict(&x);
        assert_eq!(pred, y);
    }
}
