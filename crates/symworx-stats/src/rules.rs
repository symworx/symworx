// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Threshold rules and ordered rule-list classifiers.
//!
//! Hand-authored IF–THEN logic for interpretable decisions (e.g. vitals /
//! quality / load flags). Inference is a few comparisons per rule — ideal for
//! embedded targets after rules are defined on a workstation.
//!
//! ## Pieces
//!
//! - [`ThresholdCondition`] — one feature vs threshold comparison  
//! - [`ClassificationRule`] — AND of conditions → class label  
//! - [`RuleListClassifier`] — first-match rule list + default label  
//! - [`fit_decision_stump`] — optional learned single-feature threshold (multiclass Gini)
//!
//! Pure Rust — no `linalg`.
//!
//! **Export / on-device inference:** see `docs/model_export.md` (rules are
//! especially natural on MCU and mobile).

use std::fmt;

use ndarray::Array2;

/// Comparison operator for a continuous feature threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    /// `x[j] < t`
    Lt,
    /// `x[j] <= t`
    Le,
    /// `x[j] > t`
    Gt,
    /// `x[j] >= t`
    Ge,
    /// `x[j]` approximately equal to `t` (`|x−t| <= atol`)
    Eq,
}

impl Comparison {
    fn eval(self, value: f64, threshold: f64, atol: f64) -> bool {
        match self {
            Comparison::Lt => value < threshold,
            Comparison::Le => value <= threshold,
            Comparison::Gt => value > threshold,
            Comparison::Ge => value >= threshold,
            Comparison::Eq => (value - threshold).abs() <= atol,
        }
    }
}

impl fmt::Display for Comparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Comparison::Lt => write!(f, "<"),
            Comparison::Le => write!(f, "<="),
            Comparison::Gt => write!(f, ">"),
            Comparison::Ge => write!(f, ">="),
            Comparison::Eq => write!(f, "=="),
        }
    }
}

/// One atomic condition: `x[feature] ⟨op⟩ threshold`.
#[derive(Debug, Clone, PartialEq)]
pub struct ThresholdCondition {
    /// Feature column index.
    pub feature: usize,
    /// Comparison operator.
    pub op: Comparison,
    /// Threshold value (same units as the feature).
    pub threshold: f64,
    /// Absolute tolerance for [`Comparison::Eq`] (default `1e-12` when built via helpers).
    pub atol: f64,
}

impl ThresholdCondition {
    /// Build a condition with default equality tolerance.
    pub fn new(feature: usize, op: Comparison, threshold: f64) -> Self {
        Self {
            feature,
            op,
            threshold,
            atol: 1e-12,
        }
    }

    /// Evaluate on a single feature row.
    pub fn matches(&self, row: &[f64]) -> bool {
        assert!(
            self.feature < row.len(),
            "feature {} out of range (row len {})",
            self.feature,
            row.len()
        );
        self.op.eval(row[self.feature], self.threshold, self.atol)
    }
}

impl fmt::Display for ThresholdCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "x[{}] {} {}", self.feature, self.op, self.threshold)
    }
}

/// A conjunctive rule: if **all** conditions hold, assign `label`.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassificationRule {
    /// AND-ed conditions (empty ⇒ always matches).
    pub conditions: Vec<ThresholdCondition>,
    /// Class label when the rule fires.
    pub label: usize,
    /// Optional human-readable name (for demos / export).
    pub name: Option<String>,
}

impl ClassificationRule {
    /// Single-threshold rule (decision stump style).
    pub fn threshold(feature: usize, op: Comparison, threshold: f64, label: usize) -> Self {
        Self {
            conditions: vec![ThresholdCondition::new(feature, op, threshold)],
            label,
            name: None,
        }
    }

    /// Named rule for documentation / logging.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Whether all conditions match `row`.
    pub fn matches(&self, row: &[f64]) -> bool {
        self.conditions.iter().all(|c| c.matches(row))
    }
}

impl fmt::Display for ClassificationRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref n) = self.name {
            write!(f, "{n}: ")?;
        }
        if self.conditions.is_empty() {
            write!(f, "TRUE")?;
        } else {
            for (i, c) in self.conditions.iter().enumerate() {
                if i > 0 {
                    write!(f, " AND ")?;
                }
                write!(f, "{c}")?;
            }
        }
        write!(f, " → {}", self.label)
    }
}

/// Which rule fired for a prediction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleHit {
    /// Rule at this index in [`RuleListClassifier::rules`].
    Rule(usize),
    /// Fell through to [`RuleListClassifier::default_label`].
    Default,
}

/// First-match ordered rule list + default label.
///
/// Rules are tried in order; the first matching rule’s label is returned.
/// If none match, `default_label` is used.
#[derive(Debug, Clone)]
pub struct RuleListClassifier {
    /// Ordered rules (first match wins).
    pub rules: Vec<ClassificationRule>,
    /// Label when no rule matches.
    pub default_label: usize,
    /// Optional declared class set (for proba / reporting); inferred if empty.
    pub classes: Vec<usize>,
}

impl RuleListClassifier {
    /// Build a classifier; `classes` may be empty (inferred from rules + default).
    pub fn new(rules: Vec<ClassificationRule>, default_label: usize, classes: impl Into<Vec<usize>>) -> Self {
        let mut classes = classes.into();
        if classes.is_empty() {
            classes = infer_classes(&rules, default_label);
        } else {
            classes.sort_unstable();
            classes.dedup();
        }
        Self {
            rules,
            default_label,
            classes,
        }
    }

    /// Number of rules (excluding default).
    pub fn n_rules(&self) -> usize {
        self.rules.len()
    }

    /// Predict labels for each row of `x`.
    pub fn predict(&self, x: &Array2<f64>) -> Vec<usize> {
        (0..x.nrows())
            .map(|i| {
                let row: Vec<f64> = x.row(i).to_vec();
                self.predict_one(&row)
            })
            .collect()
    }

    /// Predict a single row.
    pub fn predict_one(&self, row: &[f64]) -> usize {
        self.predict_one_with_hit(row).0
    }

    /// Predict and report which rule fired.
    pub fn predict_one_with_hit(&self, row: &[f64]) -> (usize, RuleHit) {
        for (i, rule) in self.rules.iter().enumerate() {
            if rule.matches(row) {
                return (rule.label, RuleHit::Rule(i));
            }
        }
        (self.default_label, RuleHit::Default)
    }

    /// Hits for each sample (parallel to [`predict`]).
    pub fn predict_with_hits(&self, x: &Array2<f64>) -> (Vec<usize>, Vec<RuleHit>) {
        let mut labels = Vec::with_capacity(x.nrows());
        let mut hits = Vec::with_capacity(x.nrows());
        for i in 0..x.nrows() {
            let row: Vec<f64> = x.row(i).to_vec();
            let (y, h) = self.predict_one_with_hit(&row);
            labels.push(y);
            hits.push(h);
        }
        (labels, hits)
    }

    /// Hard one-hot style probabilities: 1.0 on predicted class, 0 elsewhere.
    ///
    /// Rule lists are not probabilistic; this is for metric APIs that want a
    /// score matrix (e.g. crude ROC). Prefer real proba models for ranking.
    pub fn predict_proba_one_hot(&self, x: &Array2<f64>) -> Array2<f64> {
        let k = self.classes.len();
        let mut proba = Array2::<f64>::zeros((x.nrows(), k.max(1)));
        if k == 0 {
            return proba;
        }
        let pred = self.predict(x);
        for (i, &yi) in pred.iter().enumerate() {
            if let Some(c) = self.classes.iter().position(|&c| c == yi) {
                proba[[i, c]] = 1.0;
            }
        }
        proba
    }

    /// Accuracy on labeled data.
    pub fn accuracy(&self, x: &Array2<f64>, y: &[usize]) -> f64 {
        assert_eq!(x.nrows(), y.len());
        if y.is_empty() {
            return f64::NAN;
        }
        let pred = self.predict(x);
        pred.iter().zip(y.iter()).filter(|(a, b)| a == b).count() as f64 / y.len() as f64
    }
}

impl fmt::Display for RuleListClassifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "RuleListClassifier ({} rules):", self.rules.len())?;
        for (i, r) in self.rules.iter().enumerate() {
            writeln!(f, "  [{i}] {r}")?;
        }
        write!(f, "  default → {}", self.default_label)
    }
}

fn infer_classes(rules: &[ClassificationRule], default_label: usize) -> Vec<usize> {
    let mut c: Vec<usize> = rules.iter().map(|r| r.label).collect();
    c.push(default_label);
    c.sort_unstable();
    c.dedup();
    c
}

// ---------------------------------------------------------------------------
// Learned decision stump (optional)
// ---------------------------------------------------------------------------

/// A fitted single-feature threshold classifier (binary split → two leaves).
#[derive(Debug, Clone)]
pub struct DecisionStump {
    /// Feature used for the split.
    pub feature: usize,
    /// Split threshold: left is `x[feature] <= threshold`.
    pub threshold: f64,
    /// Label when `x[feature] <= threshold`.
    pub left_label: usize,
    /// Label when `x[feature] > threshold`.
    pub right_label: usize,
    /// Impurity of the split (weighted Gini of children); lower is better.
    pub impurity: f64,
}

impl DecisionStump {
    /// Convert to a two-rule list (left first, then right as default-free pair).
    ///
    /// Actually: rule1: feature <= t → left; default → right covers the rest.
    pub fn to_rule_list(&self) -> RuleListClassifier {
        let rule = ClassificationRule::threshold(self.feature, Comparison::Le, self.threshold, self.left_label)
            .with_name("stump_left");
        RuleListClassifier::new(vec![rule], self.right_label, [self.left_label, self.right_label])
    }

    /// Predict labels.
    pub fn predict(&self, x: &Array2<f64>) -> Vec<usize> {
        (0..x.nrows())
            .map(|i| {
                if x[[i, self.feature]] <= self.threshold {
                    self.left_label
                } else {
                    self.right_label
                }
            })
            .collect()
    }
}

/// Fit a multiclass decision stump by exhaustive threshold search (Gini).
///
/// Candidate thresholds are midpoints between sorted unique feature values.
/// Returns `None` if `x` is empty or no valid split exists.
pub fn fit_decision_stump(x: &Array2<f64>, y: &[usize]) -> Option<DecisionStump> {
    assert_eq!(x.nrows(), y.len());
    if x.nrows() < 2 || x.ncols() == 0 {
        return None;
    }

    let mut best: Option<DecisionStump> = None;

    for j in 0..x.ncols() {
        let mut pairs: Vec<(f64, usize)> = (0..x.nrows()).map(|i| (x[[i, j]], y[i])).collect();
        pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Unique midpoints
        let mut thresholds = Vec::new();
        for w in pairs.windows(2) {
            if (w[1].0 - w[0].0).abs() > 1e-15 {
                thresholds.push(0.5 * (w[0].0 + w[1].0));
            }
        }
        if thresholds.is_empty() {
            continue;
        }

        for &t in &thresholds {
            let mut left_y = Vec::new();
            let mut right_y = Vec::new();
            for i in 0..x.nrows() {
                if x[[i, j]] <= t {
                    left_y.push(y[i]);
                } else {
                    right_y.push(y[i]);
                }
            }
            if left_y.is_empty() || right_y.is_empty() {
                continue;
            }
            let n = x.nrows() as f64;
            let imp = (left_y.len() as f64 / n) * gini(&left_y) + (right_y.len() as f64 / n) * gini(&right_y);
            let left_label = majority(&left_y);
            let right_label = majority(&right_y);

            let cand = DecisionStump {
                feature: j,
                threshold: t,
                left_label,
                right_label,
                impurity: imp,
            };
            best = Some(match best {
                None => cand,
                Some(ref b) if imp < b.impurity - 1e-15 => cand,
                Some(b) => b,
            });
        }
    }

    best
}

fn gini(labels: &[usize]) -> f64 {
    if labels.is_empty() {
        return 0.0;
    }
    let n = labels.len() as f64;
    let mut classes = labels.to_vec();
    classes.sort_unstable();
    classes.dedup();
    let mut g = 1.0;
    for c in classes {
        let cnt = labels.iter().filter(|&&y| y == c).count() as f64;
        let p = cnt / n;
        g -= p * p;
    }
    g
}

fn majority(labels: &[usize]) -> usize {
    let mut classes = labels.to_vec();
    classes.sort_unstable();
    classes.dedup();
    let mut best = classes[0];
    let mut best_n = 0usize;
    for c in classes {
        let n = labels.iter().filter(|&&y| y == c).count();
        if n > best_n || (n == best_n && c < best) {
            best_n = n;
            best = c;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    #[test]
    fn single_threshold_rule() {
        let clf = RuleListClassifier::new(
            vec![ClassificationRule::threshold(0, Comparison::Ge, 0.5, 1).with_name("high")],
            0,
            [0, 1],
        );
        let x = array![[0.2], [0.5], [0.9]];
        assert_eq!(clf.predict(&x), vec![0, 1, 1]);
    }

    #[test]
    fn conjunctive_and_order() {
        // IF x0 >= 1 AND x1 < 0 → class 2; else IF x0 >= 1 → class 1; else 0
        let rules = vec![
            ClassificationRule {
                conditions: vec![
                    ThresholdCondition::new(0, Comparison::Ge, 1.0),
                    ThresholdCondition::new(1, Comparison::Lt, 0.0),
                ],
                label: 2,
                name: Some("A".into()),
            },
            ClassificationRule::threshold(0, Comparison::Ge, 1.0, 1).with_name("B"),
        ];
        let clf = RuleListClassifier::new(rules, 0, [0, 1, 2]);
        let x = array![
            [0.0, 0.0],  // default 0
            [1.5, 1.0],  // rule B → 1
            [1.5, -1.0], // rule A → 2
        ];
        assert_eq!(clf.predict(&x), vec![0, 1, 2]);
        let (_, hits) = clf.predict_with_hits(&x);
        assert_eq!(hits[0], RuleHit::Default);
        assert_eq!(hits[1], RuleHit::Rule(1));
        assert_eq!(hits[2], RuleHit::Rule(0));
    }

    #[test]
    fn decision_stump_separates() {
        let x = array![[0.0], [0.1], [0.2], [0.8], [0.9], [1.0],];
        let y = vec![0, 0, 0, 1, 1, 1];
        let stump = fit_decision_stump(&x, &y).expect("stump");
        assert_eq!(stump.feature, 0);
        assert!(stump.threshold > 0.2 && stump.threshold < 0.8);
        let pred = stump.predict(&x);
        assert_eq!(pred, y);
        let list = stump.to_rule_list();
        assert_eq!(list.predict(&x), y);
    }

    #[test]
    fn display_rule() {
        let r = ClassificationRule::threshold(1, Comparison::Lt, 3.5, 0).with_name("low_hr");
        let s = format!("{r}");
        assert!(s.contains("low_hr"));
        assert!(s.contains("x[1]"));
    }
}
