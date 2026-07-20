// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Classification evaluation metrics.
//!
//! Complements [`crate::error_metrics`] (continuous predicted-vs-expected).
//! Labels are integer class indices in `0..n_classes`. For binary logistic
//! outputs stored as `f64` `{0.0, 1.0}`, use [`labels_from_binary_f64`].
//!
//! Pure Rust — no `linalg` / LAPACK. Suitable for workstation teaching demos
//! and for scoring models whose coefficients later ship to embedded targets.

use std::fmt;

use ndarray::Array2;

/// Convert binary labels stored as `f64` (`0.0` / `1.0`) to `usize` indices.
///
/// # Panics
/// Panics if any value is not exactly `0.0` or `1.0`.
pub fn labels_from_binary_f64(y: &[f64]) -> Vec<usize> {
    y.iter()
        .enumerate()
        .map(|(i, &v)| {
            if v == 0.0 {
                0
            } else if v == 1.0 {
                1
            } else {
                panic!("y[{i}] = {v} is not binary 0.0/1.0");
            }
        })
        .collect()
}

/// Fraction of exact label matches.
///
/// Returns `f64::NAN` if lengths differ or either slice is empty.
pub fn accuracy(y_true: &[usize], y_pred: &[usize]) -> f64 {
    if y_true.len() != y_pred.len() || y_true.is_empty() {
        return f64::NAN;
    }
    let correct = y_true
        .iter()
        .zip(y_pred.iter())
        .filter(|(a, p)| a == p)
        .count();
    correct as f64 / y_true.len() as f64
}

/// Infer `n_classes` as `1 + max(label)` over both slices (empty → 0).
pub fn n_classes_from_labels(y_true: &[usize], y_pred: &[usize]) -> usize {
    y_true
        .iter()
        .chain(y_pred.iter())
        .copied()
        .max()
        .map(|m| m + 1)
        .unwrap_or(0)
}

/// Confusion matrix `C[i, j]` = count of true class `i` predicted as `j`.
///
/// Shape is `n_classes × n_classes`. If `n_classes` is `None`, it is inferred
/// via [`n_classes_from_labels`].
///
/// Returns an empty `0×0` matrix if inputs are empty or lengths differ.
pub fn confusion_matrix(
    y_true: &[usize],
    y_pred: &[usize],
    n_classes: Option<usize>,
) -> Array2<usize> {
    if y_true.len() != y_pred.len() || y_true.is_empty() {
        return Array2::zeros((0, 0));
    }
    let k = n_classes.unwrap_or_else(|| n_classes_from_labels(y_true, y_pred));
    let mut c = Array2::<usize>::zeros((k, k));
    for (&t, &p) in y_true.iter().zip(y_pred.iter()) {
        if t < k && p < k {
            c[[t, p]] += 1;
        }
    }
    c
}

/// Per-class precision `TP / (TP + FP)` (0 if denominator is 0).
pub fn precision_per_class(cm: &Array2<usize>) -> Vec<f64> {
    let k = cm.nrows();
    let mut out = vec![0.0; k];
    for j in 0..k {
        let mut col_sum = 0usize;
        let mut tp = 0usize;
        for i in 0..k {
            col_sum += cm[[i, j]];
            if i == j {
                tp = cm[[i, j]];
            }
        }
        out[j] = if col_sum == 0 {
            0.0
        } else {
            tp as f64 / col_sum as f64
        };
    }
    out
}

/// Per-class recall (sensitivity) `TP / (TP + FN)` (0 if denominator is 0).
pub fn recall_per_class(cm: &Array2<usize>) -> Vec<f64> {
    let k = cm.nrows();
    let mut out = vec![0.0; k];
    for i in 0..k {
        let mut row_sum = 0usize;
        let mut tp = 0usize;
        for j in 0..k {
            row_sum += cm[[i, j]];
            if i == j {
                tp = cm[[i, j]];
            }
        }
        out[i] = if row_sum == 0 {
            0.0
        } else {
            tp as f64 / row_sum as f64
        };
    }
    out
}

/// Per-class F1 = harmonic mean of precision and recall (0 if both are 0).
pub fn f1_per_class(precision: &[f64], recall: &[f64]) -> Vec<f64> {
    precision
        .iter()
        .zip(recall.iter())
        .map(|(&p, &r)| {
            if p + r == 0.0 {
                0.0
            } else {
                2.0 * p * r / (p + r)
            }
        })
        .collect()
}

/// Macro-average (unweighted mean) of a per-class score vector.
pub fn macro_average(scores: &[f64]) -> f64 {
    if scores.is_empty() {
        return f64::NAN;
    }
    scores.iter().sum::<f64>() / scores.len() as f64
}

/// Balanced accuracy: mean of per-class recall (handles class imbalance).
pub fn balanced_accuracy(cm: &Array2<usize>) -> f64 {
    macro_average(&recall_per_class(cm))
}

/// Binary (positive class = 1) precision / recall / F1 from labels in `{0, 1}`.
///
/// Returns `(precision, recall, f1)` or `(NaN, NaN, NaN)` if invalid.
pub fn binary_precision_recall_f1(y_true: &[usize], y_pred: &[usize]) -> (f64, f64, f64) {
    if y_true.len() != y_pred.len() || y_true.is_empty() {
        return (f64::NAN, f64::NAN, f64::NAN);
    }
    let cm = confusion_matrix(y_true, y_pred, Some(2));
    let p = precision_per_class(&cm);
    let r = recall_per_class(&cm);
    let f = f1_per_class(&p, &r);
    (p[1], r[1], f[1])
}

/// Bundle of multiclass classification metrics.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassificationReport {
    /// Number of samples.
    pub n: usize,
    /// Number of classes used for the confusion matrix.
    pub n_classes: usize,
    /// Overall accuracy.
    pub accuracy: f64,
    /// Mean of per-class recall.
    pub balanced_accuracy: f64,
    /// Macro-averaged precision.
    pub macro_precision: f64,
    /// Macro-averaged recall.
    pub macro_recall: f64,
    /// Macro-averaged F1.
    pub macro_f1: f64,
    /// Per-class precision.
    pub precision: Vec<f64>,
    /// Per-class recall.
    pub recall: Vec<f64>,
    /// Per-class F1.
    pub f1: Vec<f64>,
    /// Confusion matrix (true × predicted).
    pub confusion: Array2<usize>,
}

impl ClassificationReport {
    fn invalid() -> Self {
        Self {
            n: 0,
            n_classes: 0,
            accuracy: f64::NAN,
            balanced_accuracy: f64::NAN,
            macro_precision: f64::NAN,
            macro_recall: f64::NAN,
            macro_f1: f64::NAN,
            precision: Vec::new(),
            recall: Vec::new(),
            f1: Vec::new(),
            confusion: Array2::zeros((0, 0)),
        }
    }
}

impl fmt::Display for ClassificationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.n == 0 {
            return write!(f, "ClassificationReport(invalid / empty)");
        }
        write!(
            f,
            "n={}  classes={}  acc={:.4}  bal_acc={:.4}  macro_P={:.4}  macro_R={:.4}  macro_F1={:.4}",
            self.n,
            self.n_classes,
            self.accuracy,
            self.balanced_accuracy,
            self.macro_precision,
            self.macro_recall,
            self.macro_f1
        )
    }
}

/// Build a [`ClassificationReport`] from integer labels.
///
/// If `n_classes` is `None`, it is inferred from the labels.
pub fn classification_report(
    y_true: &[usize],
    y_pred: &[usize],
    n_classes: Option<usize>,
) -> ClassificationReport {
    if y_true.len() != y_pred.len() || y_true.is_empty() {
        return ClassificationReport::invalid();
    }
    let k = n_classes.unwrap_or_else(|| n_classes_from_labels(y_true, y_pred));
    let cm = confusion_matrix(y_true, y_pred, Some(k));
    let precision = precision_per_class(&cm);
    let recall = recall_per_class(&cm);
    let f1 = f1_per_class(&precision, &recall);
    ClassificationReport {
        n: y_true.len(),
        n_classes: k,
        accuracy: accuracy(y_true, y_pred),
        balanced_accuracy: balanced_accuracy(&cm),
        macro_precision: macro_average(&precision),
        macro_recall: macro_average(&recall),
        macro_f1: macro_average(&f1),
        precision,
        recall,
        f1,
        confusion: cm,
    }
}

/// Convenience: report from binary `f64` labels (`0.0` / `1.0`).
pub fn classification_report_binary_f64(y_true: &[f64], y_pred: &[f64]) -> ClassificationReport {
    let yt = labels_from_binary_f64(y_true);
    let yp = labels_from_binary_f64(y_pred);
    classification_report(&yt, &yp, Some(2))
}

// ---------------------------------------------------------------------------
// ROC / AUC
// ---------------------------------------------------------------------------

/// ROC curve points and AUC for a binary problem.
///
/// `fpr[i]`, `tpr[i]` correspond to classifying as positive when
/// `score >= thresholds[i]` (higher score ⇒ more positive).
#[derive(Debug, Clone, PartialEq)]
pub struct RocCurve {
    /// False positive rates.
    pub fpr: Vec<f64>,
    /// True positive rates (recall / sensitivity).
    pub tpr: Vec<f64>,
    /// Decreasing score thresholds (plus a final point at −∞ conceptually).
    pub thresholds: Vec<f64>,
    /// Area under the ROC curve (trapezoidal).
    pub auc: f64,
}

/// Binary ROC-AUC from integer labels `{0, 1}` and real-valued scores.
///
/// Higher `scores` mean more likely class **1**. Returns `NaN` if inputs are
/// empty, lengths differ, or only one class is present.
pub fn roc_auc(y_true: &[usize], scores: &[f64]) -> f64 {
    roc_curve(y_true, scores).map(|c| c.auc).unwrap_or(f64::NAN)
}

/// Build the ROC curve and AUC for binary labels `{0, 1}`.
///
/// Returns `None` if lengths differ, empty, or only one class appears.
pub fn roc_curve(y_true: &[usize], scores: &[f64]) -> Option<RocCurve> {
    if y_true.len() != scores.len() || y_true.is_empty() {
        return None;
    }
    let mut n_pos = 0usize;
    let mut n_neg = 0usize;
    for &y in y_true {
        match y {
            0 => n_neg += 1,
            1 => n_pos += 1,
            _ => return None, // only binary 0/1
        }
    }
    if n_pos == 0 || n_neg == 0 {
        return None;
    }

    // Sort by score descending
    let mut order: Vec<usize> = (0..y_true.len()).collect();
    order.sort_by(|&i, &j| {
        scores[j]
            .partial_cmp(&scores[i])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut fpr = Vec::with_capacity(y_true.len() + 2);
    let mut tpr = Vec::with_capacity(y_true.len() + 2);
    let mut thresholds = Vec::with_capacity(y_true.len() + 2);

    // Start: threshold above max score → no positives predicted
    fpr.push(0.0);
    tpr.push(0.0);
    thresholds.push(f64::INFINITY);

    let mut tp = 0usize;
    let mut fp = 0usize;
    let mut i = 0;
    while i < order.len() {
        let thr = scores[order[i]];
        // All samples with this score
        while i < order.len() && scores[order[i]] == thr {
            if y_true[order[i]] == 1 {
                tp += 1;
            } else {
                fp += 1;
            }
            i += 1;
        }
        fpr.push(fp as f64 / n_neg as f64);
        tpr.push(tp as f64 / n_pos as f64);
        thresholds.push(thr);
    }

    let auc = trapz_auc(&fpr, &tpr);
    Some(RocCurve {
        fpr,
        tpr,
        thresholds,
        auc,
    })
}

/// Trapezoidal AUC over FPR (x) and TPR (y); assumes FPR non-decreasing.
fn trapz_auc(fpr: &[f64], tpr: &[f64]) -> f64 {
    let mut area = 0.0;
    for i in 1..fpr.len() {
        let dx = fpr[i] - fpr[i - 1];
        area += dx * (tpr[i] + tpr[i - 1]) * 0.5;
    }
    area
}

/// Macro-averaged one-vs-rest ROC-AUC for multiclass labels.
///
/// `scores` is `n_samples × n_classes` (row = sample, column aligns with
/// class index `0..n_classes-1`, or pass `classes` mapping).
///
/// If `classes` is `None`, assumes labels and score columns are `0..K-1`.
/// Returns `NaN` if any class AUC is undefined (empty positive/negative set).
pub fn roc_auc_ovr(y_true: &[usize], scores: &Array2<f64>, classes: Option<&[usize]>) -> f64 {
    if y_true.len() != scores.nrows() || y_true.is_empty() {
        return f64::NAN;
    }
    let class_list: Vec<usize> = match classes {
        Some(c) => c.to_vec(),
        None => {
            let k = scores.ncols();
            (0..k).collect()
        }
    };
    if class_list.len() != scores.ncols() {
        return f64::NAN;
    }

    let mut aucs = Vec::with_capacity(class_list.len());
    for (col, &cls) in class_list.iter().enumerate() {
        let binary_y: Vec<usize> = y_true
            .iter()
            .map(|&y| if y == cls { 1 } else { 0 })
            .collect();
        let col_scores: Vec<f64> = scores.column(col).to_vec();
        let a = roc_auc(&binary_y, &col_scores);
        if !a.is_finite() {
            return f64::NAN;
        }
        aucs.push(a);
    }
    macro_average(&aucs)
}

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    #[test]
    fn perfect_binary() {
        let y = vec![0, 0, 1, 1];
        let p = vec![0, 0, 1, 1];
        assert!((accuracy(&y, &p) - 1.0).abs() < 1e-15);
        let rep = classification_report(&y, &p, Some(2));
        assert!((rep.macro_f1 - 1.0).abs() < 1e-15);
        assert_eq!(rep.confusion[[0, 0]], 2);
        assert_eq!(rep.confusion[[1, 1]], 2);
    }

    #[test]
    fn confusion_and_f1_known() {
        // true: 0,0,1,1  pred: 0,1,1,1  → TP1=2 FP1=1 FN1=0 TN=1
        let y = vec![0, 0, 1, 1];
        let p = vec![0, 1, 1, 1];
        let (prec, rec, f1) = binary_precision_recall_f1(&y, &p);
        assert!((prec - 2.0 / 3.0).abs() < 1e-12);
        assert!((rec - 1.0).abs() < 1e-12);
        let expect_f1 = 2.0 * prec * rec / (prec + rec);
        assert!((f1 - expect_f1).abs() < 1e-12);
        assert!((accuracy(&y, &p) - 0.75).abs() < 1e-12);
    }

    #[test]
    fn balanced_accuracy_imbalance() {
        // Always predict majority 0: acc high, bal_acc = 0.5
        let y = vec![0, 0, 0, 0, 1, 1];
        let p = vec![0, 0, 0, 0, 0, 0];
        let rep = classification_report(&y, &p, Some(2));
        assert!((rep.accuracy - 4.0 / 6.0).abs() < 1e-12);
        assert!((rep.balanced_accuracy - 0.5).abs() < 1e-12);
    }

    #[test]
    fn multiclass_3() {
        let y = vec![0, 1, 2, 0, 1, 2];
        let p = vec![0, 1, 2, 0, 2, 1];
        let rep = classification_report(&y, &p, Some(3));
        assert_eq!(rep.n_classes, 3);
        assert!((rep.accuracy - 4.0 / 6.0).abs() < 1e-12);
        assert_eq!(rep.confusion[[1, 2]], 1);
        assert_eq!(rep.confusion[[2, 1]], 1);
    }

    #[test]
    fn binary_f64_bridge() {
        let y = [0.0, 1.0, 1.0, 0.0];
        let p = [0.0, 1.0, 0.0, 0.0];
        let rep = classification_report_binary_f64(&y, &p);
        assert!((rep.accuracy - 0.75).abs() < 1e-12);
    }

    #[test]
    fn length_mismatch_nan() {
        assert!(accuracy(&[0, 1], &[0]).is_nan());
        assert_eq!(confusion_matrix(&[0], &[0, 1], None).nrows(), 0);
    }

    #[test]
    fn roc_auc_perfect() {
        let y = vec![0, 0, 1, 1];
        let s = vec![0.1, 0.2, 0.8, 0.9];
        let auc = roc_auc(&y, &s);
        assert!((auc - 1.0).abs() < 1e-12, "auc={auc}");
    }

    #[test]
    fn roc_auc_randomish() {
        // All scores equal → AUC 0.5
        let y = vec![0, 1, 0, 1];
        let s = vec![0.5, 0.5, 0.5, 0.5];
        let auc = roc_auc(&y, &s);
        assert!((auc - 0.5).abs() < 1e-12, "auc={auc}");
    }

    #[test]
    fn roc_auc_inverted() {
        let y = vec![0, 0, 1, 1];
        let s = vec![0.9, 0.8, 0.2, 0.1];
        let auc = roc_auc(&y, &s);
        assert!((auc - 0.0).abs() < 1e-12, "auc={auc}");
    }

    #[test]
    fn roc_auc_ovr_multiclass() {
        // 3 classes, perfect scores on diagonal
        let y = vec![0, 0, 1, 1, 2, 2];
        let scores = array![
            [0.9, 0.05, 0.05],
            [0.8, 0.1, 0.1],
            [0.1, 0.85, 0.05],
            [0.05, 0.9, 0.05],
            [0.05, 0.05, 0.9],
            [0.1, 0.1, 0.8],
        ];
        let auc = roc_auc_ovr(&y, &scores, None);
        assert!(auc > 0.99, "macro OVR auc={auc}");
    }
}
