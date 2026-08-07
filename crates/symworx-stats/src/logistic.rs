// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Logistic regression (binary + multiclass).
//!
//! **Binary:** `P(y = 1 | x) = σ(b + x · β)` via gradient descent on average
//! binary cross-entropy (optional L2 on β).
//!
//! **Multiclass:** one-vs-rest ([`logistic_regression_ovr`]) — one binary model
//! per class, probabilities renormalized across classes. Pure Rust; no
//! `linalg` / LAPACK.
//!
//! **Export / on-device inference:** see `docs/model_export.md` in this crate
//! (embedded C, iOS, Android, web).

use ndarray::{
    Array1,
    Array2,
};

/// Numerically stable logistic sigmoid `σ(z) = 1 / (1 + e^{−z})`.
#[inline]
pub fn sigmoid(z: f64) -> f64 {
    // Avoid overflow in exp for large |z|
    if z >= 0.0 {
        let ez = (-z).exp();
        1.0 / (1.0 + ez)
    } else {
        let ez = z.exp();
        ez / (1.0 + ez)
    }
}

/// Fitted binary logistic model: intercept + coefficients.
///
/// Prediction of class probabilities: `p = σ(intercept + X · coefficients)`.
#[derive(Debug, Clone)]
pub struct LogisticModel {
    /// Intercept term (bias).
    pub intercept: f64,
    /// Feature coefficients (length = n_features).
    pub coefficients: Array1<f64>,
    /// Final average negative log-likelihood (+ L2 penalty if used).
    pub loss: f64,
    /// Iterations performed.
    pub iterations: usize,
    /// `true` if the parameter-step or gradient stopping criterion was met.
    pub converged: bool,
}

impl LogisticModel {
    /// Number of features (excluding intercept).
    pub fn n_features(&self) -> usize {
        self.coefficients.len()
    }

    /// Linear predictor (logit) for each row of `x`.
    pub fn decision_function(&self, x: &Array2<f64>) -> Array1<f64> {
        assert_eq!(
            x.ncols(),
            self.coefficients.len(),
            "feature dimension mismatch: X has {} cols, model has {} coefficients",
            x.ncols(),
            self.coefficients.len()
        );
        x.dot(&self.coefficients) + self.intercept
    }

    /// Predicted class-1 probabilities for each row of `x`.
    pub fn predict_proba(&self, x: &Array2<f64>) -> Array1<f64> {
        self.decision_function(x).mapv(sigmoid)
    }

    /// Hard class labels in `{0, 1}` using `threshold` (default 0.5 via
    /// [`LogisticConfig::threshold`] at fit time is not stored — pass explicitly).
    pub fn predict(&self, x: &Array2<f64>, threshold: f64) -> Array1<f64> {
        self.predict_proba(x).mapv(|p| if p >= threshold { 1.0 } else { 0.0 })
    }

    /// Classification accuracy on labeled data (`y` in `{0, 1}`).
    pub fn accuracy(&self, x: &Array2<f64>, y: &Array1<f64>, threshold: f64) -> f64 {
        assert_eq!(x.nrows(), y.len());
        if y.is_empty() {
            return f64::NAN;
        }
        let pred = self.predict(x, threshold);
        let mut correct = 0usize;
        for i in 0..y.len() {
            if (pred[i] - y[i]).abs() < 0.5 {
                correct += 1;
            }
        }
        correct as f64 / y.len() as f64
    }
}

/// Options for [`logistic_regression`].
#[derive(Debug, Clone)]
pub struct LogisticConfig {
    /// Maximum gradient-descent iterations.
    pub max_iter: usize,
    /// Stop when `‖Δθ‖_∞ < tol` (parameter step).
    pub tol: f64,
    /// Fixed learning rate (step size).
    pub learning_rate: f64,
    /// L2 penalty strength on **coefficients only** (intercept unpenalized).
    /// `0.0` → unregularized MLE.
    pub l2: f64,
    /// Probability threshold for [`LogisticModel::predict`] helpers used in
    /// diagnostics (fit itself does not depend on this).
    pub threshold: f64,
    /// If `true`, include an intercept; otherwise intercept is fixed at 0.
    pub fit_intercept: bool,
}

impl Default for LogisticConfig {
    fn default() -> Self {
        Self {
            max_iter: 1000,
            tol: 1e-6,
            learning_rate: 0.1,
            l2: 0.0,
            threshold: 0.5,
            fit_intercept: true,
        }
    }
}

/// Fit binary logistic regression by gradient descent.
///
/// Minimizes the average binary cross-entropy
///
/// ```text
/// L = (−1/n) Σᵢ [ yᵢ log pᵢ + (1 − yᵢ) log(1 − pᵢ) ] + (λ/2) ‖β‖²
/// ```
///
/// with `pᵢ = σ(b + xᵢ · β)` and optional L2 penalty `λ = config.l2` on `β`
/// only. Analytic gradients; no matrix factorization.
///
/// # Arguments
/// * `x` — design matrix (n_samples × n_features)
/// * `y` — labels in `{0.0, 1.0}` (other values are rejected)
/// * `config` — optimizer and regularization settings
///
/// # Panics
/// Panics if `x`/`y` row counts differ, `y` is not binary, `l2 < 0`, or
/// `learning_rate ≤ 0`.
///
/// # Example
/// ```
/// use ndarray::array;
/// use symworx_stats::{logistic_regression, LogisticConfig};
///
/// let x = array![
///     [0.0], [0.1], [0.2], [0.8], [0.9], [1.0],
/// ];
/// let y = array![0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
/// let model = logistic_regression(&x, &y, &LogisticConfig::default());
/// let p = model.predict_proba(&x);
/// assert!(p[0] < 0.5 && p[5] > 0.5);
/// ```
pub fn logistic_regression(x: &Array2<f64>, y: &Array1<f64>, config: &LogisticConfig) -> LogisticModel {
    let n_samples = x.nrows();
    let n_features = x.ncols();
    assert_eq!(y.len(), n_samples, "X and y must have the same number of rows");
    assert!(config.l2 >= 0.0, "l2 must be non-negative");
    assert!(config.learning_rate > 0.0, "learning_rate must be positive");
    assert!(n_samples > 0, "need at least one sample");

    for (i, &yi) in y.iter().enumerate() {
        assert!(
            yi == 0.0 || yi == 1.0,
            "y[{i}] = {yi} is not binary (expected 0.0 or 1.0)"
        );
    }

    let n = n_samples as f64;
    let mut intercept = 0.0;
    let mut beta = Array1::<f64>::zeros(n_features);

    let mut loss = f64::INFINITY;
    let mut converged = false;
    let mut iterations = 0;

    for iter in 0..config.max_iter {
        iterations = iter + 1;

        // p_i = σ(b + x_i · β)
        let logits = x.dot(&beta) + intercept;
        let proba = logits.mapv(sigmoid);

        // residual r = p - y  (gradient of NLL w.r.t. logit)
        let residual = &proba - y;

        // ∇b = mean(r),  ∇β = Xᵀ r / n + λ β
        let grad_b = if config.fit_intercept {
            residual.mean().unwrap_or(0.0)
        } else {
            0.0
        };
        let mut grad_beta = x.t().dot(&residual) / n;
        if config.l2 > 0.0 {
            grad_beta = grad_beta + config.l2 * &beta;
        }

        // Loss for diagnostics (average NLL + ½ λ ‖β‖²)
        loss = binary_nll(&proba, y) + 0.5 * config.l2 * beta.dot(&beta);

        let step_b = config.learning_rate * grad_b;
        let step_beta = &grad_beta * config.learning_rate;

        let max_step = step_beta.iter().fold(step_b.abs(), |acc, &s| acc.max(s.abs()));

        if config.fit_intercept {
            intercept -= step_b;
        }
        beta = &beta - &step_beta;

        if max_step < config.tol {
            converged = true;
            // refresh loss after last step
            let logits = x.dot(&beta) + intercept;
            let proba = logits.mapv(sigmoid);
            loss = binary_nll(&proba, y) + 0.5 * config.l2 * beta.dot(&beta);
            break;
        }
    }

    if !converged {
        let logits = x.dot(&beta) + intercept;
        let proba = logits.mapv(sigmoid);
        loss = binary_nll(&proba, y) + 0.5 * config.l2 * beta.dot(&beta);
    }

    LogisticModel {
        intercept,
        coefficients: beta,
        loss,
        iterations,
        converged,
    }
}

/// Average binary cross-entropy with probability clipping for stability.
fn binary_nll(proba: &Array1<f64>, y: &Array1<f64>) -> f64 {
    const EPS: f64 = 1e-15;
    let n = y.len() as f64;
    if n == 0.0 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..y.len() {
        let p = proba[i].clamp(EPS, 1.0 - EPS);
        let yi = y[i];
        s += -(yi * p.ln() + (1.0 - yi) * (1.0 - p).ln());
    }
    s / n
}

/// Convenience: logistic fit with default [`LogisticConfig`].
pub fn logistic(x: &Array2<f64>, y: &Array1<f64>) -> LogisticModel {
    logistic_regression(x, y, &LogisticConfig::default())
}

// ---------------------------------------------------------------------------
// Multiclass (one-vs-rest)
// ---------------------------------------------------------------------------

/// Fitted multiclass logistic model (one-vs-rest).
///
/// Embed note: ship one `(intercept, coefficients)` pair per class; predict
/// with argmax of binary scores or renormalized probabilities.
#[derive(Debug, Clone)]
pub struct MulticlassLogisticModel {
    /// Class labels in fit order (sorted unique).
    pub classes: Vec<usize>,
    /// Binary “class k vs rest” models, aligned with [`Self::classes`].
    pub models: Vec<LogisticModel>,
}

impl MulticlassLogisticModel {
    /// Number of classes.
    pub fn n_classes(&self) -> usize {
        self.classes.len()
    }

    /// Number of features (from the first binary model).
    pub fn n_features(&self) -> usize {
        self.models.first().map(|m| m.n_features()).unwrap_or(0)
    }

    /// Whether every binary sub-model reported convergence.
    pub fn converged(&self) -> bool {
        self.models.iter().all(|m| m.converged)
    }

    /// Mean binary training loss across OVR models.
    pub fn mean_loss(&self) -> f64 {
        if self.models.is_empty() {
            return f64::NAN;
        }
        self.models.iter().map(|m| m.loss).sum::<f64>() / self.models.len() as f64
    }

    /// Decision scores (n_samples × n_classes): logit of each OVR classifier.
    pub fn decision_function(&self, x: &Array2<f64>) -> Array2<f64> {
        let k = self.n_classes();
        let mut scores = Array2::<f64>::zeros((x.nrows(), k));
        for (c, model) in self.models.iter().enumerate() {
            let s = model.decision_function(x);
            for i in 0..x.nrows() {
                scores[[i, c]] = s[i];
            }
        }
        scores
    }

    /// Class probabilities (n_samples × n_classes).
    ///
    /// Each column is the OVR `P(class = c | x)` from the binary model; rows are
    /// renormalized to sum to 1 (sklearn-style OVR probability).
    pub fn predict_proba(&self, x: &Array2<f64>) -> Array2<f64> {
        let k = self.n_classes();
        let mut proba = Array2::<f64>::zeros((x.nrows(), k));
        for (c, model) in self.models.iter().enumerate() {
            let p = model.predict_proba(x);
            for i in 0..x.nrows() {
                proba[[i, c]] = p[i];
            }
        }
        // Renormalize rows
        for i in 0..x.nrows() {
            let mut sum = 0.0;
            for c in 0..k {
                sum += proba[[i, c]];
            }
            if sum > 0.0 {
                for c in 0..k {
                    proba[[i, c]] /= sum;
                }
            } else if k > 0 {
                let u = 1.0 / k as f64;
                for c in 0..k {
                    proba[[i, c]] = u;
                }
            }
        }
        proba
    }

    /// Predicted class labels (values from [`Self::classes`]).
    ///
    /// Uses argmax of renormalized probabilities (ties → lower class index).
    pub fn predict(&self, x: &Array2<f64>) -> Vec<usize> {
        let proba = self.predict_proba(x);
        let mut out = Vec::with_capacity(x.nrows());
        for i in 0..x.nrows() {
            let mut best_c = 0usize;
            let mut best_p = f64::NEG_INFINITY;
            for c in 0..self.n_classes() {
                let p = proba[[i, c]];
                if p > best_p || (p == best_p && c < best_c) {
                    best_p = p;
                    best_c = c;
                }
            }
            out.push(self.classes[best_c]);
        }
        out
    }

    /// In-sample accuracy on integer labels.
    pub fn accuracy(&self, x: &Array2<f64>, y: &[usize]) -> f64 {
        assert_eq!(x.nrows(), y.len());
        if y.is_empty() {
            return f64::NAN;
        }
        let pred = self.predict(x);
        let correct = pred.iter().zip(y.iter()).filter(|(a, b)| a == b).count();
        correct as f64 / y.len() as f64
    }
}

/// Fit multiclass logistic regression via **one-vs-rest**.
///
/// For each class `c`, trains a binary logistic model with labels
/// `1` if `y_i == c` else `0`, reusing [`logistic_regression`].
///
/// # Arguments
/// * `x` — n_samples × n_features
/// * `y` — integer class labels (any hashable set of `usize` values)
/// * `config` — shared optimizer settings for every binary sub-model
///
/// # Panics
/// Panics if row counts differ, `x` is empty, or fewer than two classes appear.
///
/// # Example
/// ```
/// use ndarray::array;
/// use symworx_stats::{logistic_regression_ovr, LogisticConfig};
///
/// let x = array![
///     [0.0, 0.0], [0.1, 0.0],
///     [5.0, 5.0], [5.1, 5.0],
///     [0.0, 5.0], [0.1, 5.1],
/// ];
/// let y = vec![0, 0, 1, 1, 2, 2];
/// let model = logistic_regression_ovr(&x, &y, &LogisticConfig {
///     max_iter: 3000,
///     learning_rate: 0.3,
///     ..Default::default()
/// });
/// assert_eq!(model.n_classes(), 3);
/// ```
pub fn logistic_regression_ovr(x: &Array2<f64>, y: &[usize], config: &LogisticConfig) -> MulticlassLogisticModel {
    assert_eq!(x.nrows(), y.len(), "X and y length mismatch");
    assert!(x.nrows() > 0, "need at least one sample");
    assert!(x.ncols() > 0, "need at least one feature");

    let mut classes: Vec<usize> = y.to_vec();
    classes.sort_unstable();
    classes.dedup();
    assert!(
        classes.len() >= 2,
        "multiclass logistic needs at least 2 classes, got {}",
        classes.len()
    );

    let mut models = Vec::with_capacity(classes.len());
    for &cls in &classes {
        let y_bin = Array1::from(
            y.iter()
                .map(|&yi| if yi == cls { 1.0 } else { 0.0 })
                .collect::<Vec<_>>(),
        );
        models.push(logistic_regression(x, &y_bin, config));
    }

    MulticlassLogisticModel { classes, models }
}

/// OVR multiclass logistic with default [`LogisticConfig`].
pub fn logistic_ovr(x: &Array2<f64>, y: &[usize]) -> MulticlassLogisticModel {
    logistic_regression_ovr(x, y, &LogisticConfig::default())
}

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    #[test]
    fn sigmoid_bounds_and_center() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-15);
        assert!(sigmoid(50.0) > 0.999);
        assert!(sigmoid(-50.0) < 0.001);
        assert!(sigmoid(1.0) > 0.5);
        assert!(sigmoid(-1.0) < 0.5);
    }

    #[test]
    fn separates_1d_threshold() {
        // y = 1 when x > 0.5
        let x = array![[0.0], [0.1], [0.2], [0.3], [0.4], [0.6], [0.7], [0.8], [0.9], [1.0],];
        let y = array![0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0];

        let model = logistic_regression(
            &x,
            &y,
            &LogisticConfig {
                max_iter: 5000,
                learning_rate: 0.5,
                tol: 1e-8,
                ..Default::default()
            },
        );

        assert!(
            model.coefficients[0] > 0.0,
            "expected positive slope, got {:?}",
            model.coefficients
        );
        let acc = model.accuracy(&x, &y, 0.5);
        assert!(acc >= 0.9, "accuracy {acc}, loss {}", model.loss);

        let p = model.predict_proba(&x);
        assert!(p[0] < p[9], "proba should increase with x");
    }

    #[test]
    fn two_feature_linearly_separable() {
        // Class 1 roughly when x0 + x1 > 1
        let x = array![
            [0.0, 0.0],
            [0.2, 0.1],
            [0.3, 0.2],
            [0.1, 0.4],
            [0.9, 0.8],
            [0.7, 0.7],
            [0.8, 0.6],
            [1.0, 1.0],
        ];
        let y = array![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0];

        let model = logistic_regression(
            &x,
            &y,
            &LogisticConfig {
                max_iter: 8000,
                learning_rate: 0.3,
                l2: 1e-4,
                ..Default::default()
            },
        );

        let acc = model.accuracy(&x, &y, 0.5);
        assert!(acc >= 0.875, "accuracy {acc}");
        assert!(model.coefficients.iter().all(|&c| c > 0.0));
    }

    #[test]
    fn l2_shrinks_coefficients() {
        let x = array![[0.0], [0.2], [0.4], [0.6], [0.8], [1.0], [0.1], [0.9],];
        let y = array![0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0];

        let unreg = logistic_regression(
            &x,
            &y,
            &LogisticConfig {
                max_iter: 5000,
                learning_rate: 0.4,
                l2: 0.0,
                ..Default::default()
            },
        );
        let reg = logistic_regression(
            &x,
            &y,
            &LogisticConfig {
                max_iter: 5000,
                learning_rate: 0.4,
                l2: 2.0,
                ..Default::default()
            },
        );

        assert!(
            reg.coefficients[0].abs() < unreg.coefficients[0].abs(),
            "L2 should shrink |β|: unreg={} reg={}",
            unreg.coefficients[0],
            reg.coefficients[0]
        );
    }

    #[test]
    fn no_intercept_option() {
        let x = array![[-2.0], [-1.0], [1.0], [2.0]];
        let y = array![0.0, 0.0, 1.0, 1.0];
        let model = logistic_regression(
            &x,
            &y,
            &LogisticConfig {
                fit_intercept: false,
                max_iter: 3000,
                learning_rate: 0.2,
                ..Default::default()
            },
        );
        assert_eq!(model.intercept, 0.0);
        assert!(model.coefficients[0] > 0.0);
    }

    #[test]
    #[should_panic(expected = "not binary")]
    fn rejects_non_binary_labels() {
        let x = array![[0.0], [1.0]];
        let y = array![0.0, 2.0];
        let _ = logistic_regression(&x, &y, &LogisticConfig::default());
    }

    #[test]
    fn predict_shapes() {
        let x = array![[0.0], [1.0], [0.5]];
        let y = array![0.0, 1.0, 1.0];
        let model = logistic(&x, &y);
        assert_eq!(model.predict_proba(&x).len(), 3);
        assert_eq!(model.predict(&x, 0.5).len(), 3);
        assert_eq!(model.decision_function(&x).len(), 3);
    }

    #[test]
    fn ovr_three_class_blobs() {
        let x = array![
            [0.0, 0.0],
            [0.1, 0.05],
            [0.05, 0.1],
            [0.15, 0.0],
            [4.0, 4.0],
            [4.1, 3.9],
            [3.9, 4.1],
            [4.05, 4.05],
            [0.0, 4.0],
            [0.1, 4.1],
            [-0.05, 3.9],
            [0.05, 4.05],
        ];
        let y = vec![0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2];
        let model = logistic_regression_ovr(
            &x,
            &y,
            &LogisticConfig {
                max_iter: 8000,
                learning_rate: 0.35,
                l2: 0.01,
                tol: 1e-8,
                ..Default::default()
            },
        );
        assert_eq!(model.n_classes(), 3);
        assert_eq!(model.classes, vec![0, 1, 2]);
        let acc = model.accuracy(&x, &y);
        assert!(acc >= 0.9, "acc={acc}");

        let proba = model.predict_proba(&x);
        for i in 0..proba.nrows() {
            let s: f64 = proba.row(i).sum();
            assert!((s - 1.0).abs() < 1e-10, "row {i} sum={s}");
        }
        let pred = model.predict(&x);
        assert_eq!(pred.len(), y.len());
    }

    #[test]
    fn ovr_binary_reduces() {
        let x = array![[0.0], [0.1], [0.2], [0.8], [0.9], [1.0]];
        let y = vec![0, 0, 0, 1, 1, 1];
        let model = logistic_regression_ovr(
            &x,
            &y,
            &LogisticConfig {
                max_iter: 5000,
                learning_rate: 0.4,
                ..Default::default()
            },
        );
        assert_eq!(model.n_classes(), 2);
        assert!(model.accuracy(&x, &y) >= 0.9);
    }
}
