// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Shared local-evaluation envelope for exported models.
//!
//! Predict parameters stay **per kind** (`LinearModel`, logistic, pulse-response,
//! Kalman matrices, …). Evaluation does **not**: every model ships the same
//! [`EvalPolicy`] (thresholds + residual reference) and emits the same
//! [`EvalReport`] (flag + percentile) so mobile/cloud can parse a packet
//! without a per-model schema.
//!
//! [`ModelEval`] is the down-bundle section: policy plus optional training-time
//! [`RegressionReport`]. Runtime apply is [`EvalPolicy::evaluate`] — a scalar
//! `score` versus thresholds, and an optional Gaussian CDF for percentile rank.
//!
//! JSON field names (enable the `serde` feature to derive them) are the
//! canonical wire shape. See `docs/model_export.md`.

use crate::{
    RegressionReport,
    mae,
    mean,
    percentile,
    regression_report,
    residuals,
    std_dev_sample,
};

/// Three-level flag. Same vocabulary as `symworx-embed::VitalsStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum EvalFlag {
    /// Score below the warning threshold (or eval skipped / invalid).
    Normal,
    /// Score at or above warning, below critical.
    Warning,
    /// Score at or above the critical threshold.
    Critical,
}

impl EvalFlag {
    /// Wire label (`normal`, `warning`, `critical`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }
}

/// How a local window becomes the scalar compared to [`EvalThresholds`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum EvalMode {
    /// `score` is a residual magnitude (MAE / RMSE / `|e|` / Kalman innovation).
    Residual,
    /// `score` is a class probability, surprise (`1 − p`), or margin.
    Score,
    /// `score` is a raw feature versus a shipped band (HR, RMSSD, DET, …).
    Band,
}

/// Warning / critical cuts on the device `score`. Higher is worse.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EvalThresholds {
    /// `score >= warning` → [`EvalFlag::Warning`] (unless critical).
    pub warning: f64,
    /// `score >= critical` → [`EvalFlag::Critical`].
    pub critical: f64,
}

/// Shipped reference distribution of the **score** (not the raw series).
///
/// Percentile rank is vs this reference, not a live local rolling rank.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "snake_case"))]
pub enum ResidualRef {
    /// No percentile; [`EvalReport::pct`] stays `None`.
    None,
    /// `pct = 100 · Φ((score − mean) / sd)`.
    Gaussian {
        /// Mean of the training scores.
        mean: f64,
        /// Sample SD of the training scores (`n − 1`).
        sd: f64,
    },
}

/// Runtime policy shipped **down** with every model kind.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EvalPolicy {
    /// Stable id copied onto every [`EvalReport`].
    pub model_id: String,
    /// Monotonic bundle version (retrain / rollback).
    pub version: u32,
    /// Which family of `score` this policy expects.
    pub mode: EvalMode,
    /// Flag cuts on `score`.
    pub thresholds: EvalThresholds,
    /// Optional CDF for percentile rank.
    pub residual_ref: ResidualRef,
    /// Minimum window length; shorter windows → `valid = false`.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub window_n: Option<usize>,
    /// Skip eval when quality is missing or below this (SQI / peak confidence).
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub min_quality: Option<f64>,
}

/// Down-bundle eval section: one policy + optional lab fit metrics.
///
/// Training errors (`fit`) are metadata for upstream dashboards. They are
/// **not** consulted by [`EvalPolicy::evaluate`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ModelEval {
    /// Thresholds, residual reference, window, identity.
    pub policy: EvalPolicy,
    /// Hold-out (or train) [`RegressionReport`]. Omit rather than send `n = 0`.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub fit: Option<RegressionReport>,
}

/// Compact result shipped **up** (edge → mobile, mobile → cloud).
///
/// Envelope fields (`sid`, `ts`) belong on the packet, not here.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EvalReport {
    /// Same as [`EvalPolicy::model_id`].
    pub model_id: String,
    /// Same as [`EvalPolicy::version`].
    pub version: u32,
    /// Copied from the policy so parsers need not look up the bundle.
    pub mode: EvalMode,
    /// Whether the window and quality checks passed.
    pub valid: bool,
    /// Actionable flag. Invalid reports are [`EvalFlag::Normal`] so they do not trigger.
    pub flag: EvalFlag,
    /// 0–100 rank vs [`ResidualRef`]. `None` if no reference or invalid.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub pct: Option<f64>,
    /// Samples in the evaluated window (`0` if inputs were unusable).
    pub n: usize,
    /// Optional quality 0–1 (SQI).
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub quality: Option<f64>,
    /// Scalar that was compared to thresholds. Optional on constrained links.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub score: Option<f64>,
}

impl EvalPolicy {
    /// Build a residual-mode policy from signed residuals `e = y − ŷ`.
    ///
    /// Thresholds and the Gaussian reference are fit on `|e|` (the score).
    /// `warning_percentile` / `critical_percentile` are in `[0, 100]`
    /// (typical: 90 and 99).
    pub fn from_abs_residuals(
        model_id: impl Into<String>,
        version: u32,
        residuals: &[f64],
        warning_percentile: f64,
        critical_percentile: f64,
    ) -> Self {
        let abs_e: Vec<f64> = residuals.iter().map(|e| e.abs()).collect();
        let cuts = percentile(&abs_e, vec![warning_percentile, critical_percentile]);
        let residual_ref = if abs_e.len() >= 2 {
            let sd = std_dev_sample(&abs_e);
            if sd.is_finite() && sd > 0.0 {
                ResidualRef::Gaussian { mean: mean(&abs_e), sd }
            } else {
                ResidualRef::None
            }
        } else {
            ResidualRef::None
        };
        Self {
            model_id: model_id.into(),
            version,
            mode: EvalMode::Residual,
            thresholds: EvalThresholds {
                warning: cuts[0],
                critical: cuts[1],
            },
            residual_ref,
            window_n: None,
            min_quality: None,
        }
    }

    /// Flag for a finite `score`. NaN → [`EvalFlag::Normal`].
    pub fn flag_for(&self, score: f64) -> EvalFlag {
        if !score.is_finite() {
            return EvalFlag::Normal;
        }
        if score >= self.thresholds.critical {
            EvalFlag::Critical
        } else if score >= self.thresholds.warning {
            EvalFlag::Warning
        } else {
            EvalFlag::Normal
        }
    }

    /// Percentile 0–100 vs [`ResidualRef`]. `None` if the reference is unusable.
    pub fn percentile(&self, score: f64) -> Option<f64> {
        if !score.is_finite() {
            return None;
        }
        match self.residual_ref {
            ResidualRef::None => None,
            ResidualRef::Gaussian { mean, sd } => {
                if !mean.is_finite() || !sd.is_finite() || sd <= 0.0 {
                    return None;
                }
                let z = (score - mean) / sd;
                let p = standard_normal_cdf(z);
                if p.is_finite() {
                    Some((p * 100.0).clamp(0.0, 100.0))
                } else {
                    None
                }
            }
        }
    }

    /// Apply the policy to a precomputed `score`.
    ///
    /// Invalid windows (`n` too small, quality too low, non-finite score)
    /// return `valid = false`, `flag = normal`, `pct = None`.
    pub fn evaluate(&self, score: f64, n: usize, quality: Option<f64>) -> EvalReport {
        let window_ok = self.window_n.map(|need| n >= need).unwrap_or(true);
        let quality_ok = match self.min_quality {
            None => true,
            Some(min_q) => quality.map(|q| q.is_finite() && q >= min_q).unwrap_or(false),
        };
        let valid = window_ok && quality_ok && score.is_finite() && n > 0;
        if !valid {
            return EvalReport {
                model_id: self.model_id.clone(),
                version: self.version,
                mode: self.mode,
                valid: false,
                flag: EvalFlag::Normal,
                pct: None,
                n,
                quality,
                score: score.is_finite().then_some(score),
            };
        }
        EvalReport {
            model_id: self.model_id.clone(),
            version: self.version,
            mode: self.mode,
            valid: true,
            flag: self.flag_for(score),
            pct: self.percentile(score),
            n,
            quality,
            score: Some(score),
        }
    }
}

impl ModelEval {
    /// Policy only (no lab fit metrics).
    pub fn from_policy(policy: EvalPolicy) -> Self {
        Self { policy, fit: None }
    }

    /// Policy plus a fit report. Drops invalid (`n = 0`) reports.
    pub fn with_fit(policy: EvalPolicy, fit: RegressionReport) -> Self {
        let fit = if fit.n == 0 { None } else { Some(fit) };
        Self { policy, fit }
    }

    /// [`EvalPolicy::evaluate`] on the inner policy.
    pub fn evaluate(&self, score: f64, n: usize, quality: Option<f64>) -> EvalReport {
        self.policy.evaluate(score, n, quality)
    }
}

/// Residual-mode helper: `score = MAE(actual, predicted)`.
///
/// Length mismatch or empty inputs → invalid report (`n = 0`).
pub fn evaluate_residuals(policy: &EvalPolicy, actual: &[f64], predicted: &[f64], quality: Option<f64>) -> EvalReport {
    let score = mae(actual, predicted);
    let n = if score.is_nan() { 0 } else { actual.len() };
    policy.evaluate(score, n, quality)
}

/// Author a [`ModelEval`] from paired `y` / `ŷ`: policy on residuals plus fit metrics.
pub fn model_eval_from_predictions(
    model_id: impl Into<String>,
    version: u32,
    actual: &[f64],
    predicted: &[f64],
    warning_percentile: f64,
    critical_percentile: f64,
) -> ModelEval {
    let e = residuals(actual, predicted);
    let policy = EvalPolicy::from_abs_residuals(model_id, version, &e, warning_percentile, critical_percentile);
    ModelEval::with_fit(policy, regression_report(actual, predicted))
}

/// Abramowitz & Stegun 7.1.26; |err| ≲ 1.5e-7. Enough for percentile ranks.
fn erf_approx(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let ax = x.abs();
    let t = 1.0 / (1.0 + 0.327_591_1 * ax);
    let y = 1.0
        - (((((1.061_405_429_f64).mul_add(t, -1.453_152_027)).mul_add(t, 1.421_413_741)).mul_add(t, -0.284_496_736))
            .mul_add(t, 0.254_829_592))
            * t
            * (-ax * ax).exp();
    sign * y
}

fn standard_normal_cdf(z: f64) -> f64 {
    if !z.is_finite() {
        return f64::NAN;
    }
    0.5 * (1.0 + erf_approx(z / std::f64::consts::SQRT_2))
}

impl std::fmt::Display for EvalReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}@v{} flag={} valid={} n={}",
            self.model_id,
            self.version,
            self.flag.as_str(),
            self.valid,
            self.n
        )?;
        if let Some(pct) = self.pct {
            write!(f, " pct={pct:.1}")?;
        }
        if let Some(score) = self.score {
            write!(f, " score={score:.4}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> EvalPolicy {
        EvalPolicy {
            model_id: "hr_rest_v1".into(),
            version: 1,
            mode: EvalMode::Residual,
            thresholds: EvalThresholds {
                warning: 5.0,
                critical: 12.0,
            },
            residual_ref: ResidualRef::Gaussian { mean: 2.0, sd: 1.0 },
            window_n: Some(3),
            min_quality: Some(0.5),
        }
    }

    #[test]
    fn flag_bands() {
        let p = policy();
        assert_eq!(p.flag_for(4.9), EvalFlag::Normal);
        assert_eq!(p.flag_for(5.0), EvalFlag::Warning);
        assert_eq!(p.flag_for(11.9), EvalFlag::Warning);
        assert_eq!(p.flag_for(12.0), EvalFlag::Critical);
        assert_eq!(p.flag_for(f64::NAN), EvalFlag::Normal);
    }

    #[test]
    fn gaussian_percentile_known() {
        let p = policy();
        // score = mean → 50th
        let pct = p.percentile(2.0).unwrap();
        assert!((pct - 50.0).abs() < 0.05, "pct={pct}");
        // +1 sd → ~84.13
        let pct = p.percentile(3.0).unwrap();
        assert!((pct - 84.13).abs() < 0.05, "pct={pct}");
        let none = EvalPolicy {
            residual_ref: ResidualRef::None,
            ..policy()
        };
        assert!(none.percentile(2.0).is_none());
    }

    #[test]
    fn evaluate_valid_warning() {
        let r = policy().evaluate(6.0, 10, Some(0.9));
        assert!(r.valid);
        assert_eq!(r.flag, EvalFlag::Warning);
        assert!(r.pct.is_some());
        assert_eq!(r.score, Some(6.0));
        assert_eq!(r.model_id, "hr_rest_v1");
    }

    #[test]
    fn short_window_is_invalid() {
        let r = policy().evaluate(20.0, 2, Some(0.9));
        assert!(!r.valid);
        assert_eq!(r.flag, EvalFlag::Normal);
        assert!(r.pct.is_none());
    }

    #[test]
    fn low_quality_is_invalid() {
        let r = policy().evaluate(20.0, 10, Some(0.1));
        assert!(!r.valid);
        assert_eq!(r.flag, EvalFlag::Normal);
    }

    #[test]
    fn missing_quality_with_min_is_invalid() {
        let r = policy().evaluate(20.0, 10, None);
        assert!(!r.valid);
    }

    #[test]
    fn from_abs_residuals_sets_thresholds() {
        let e = [0.0, 1.0, -1.0, 2.0, -2.0, 10.0];
        let p = EvalPolicy::from_abs_residuals("m", 1, &e, 50.0, 100.0);
        assert_eq!(p.mode, EvalMode::Residual);
        assert!(p.thresholds.warning.is_finite());
        assert!(p.thresholds.critical >= p.thresholds.warning);
        match p.residual_ref {
            ResidualRef::Gaussian { mean, sd } => {
                assert!(mean > 0.0);
                assert!(sd > 0.0);
            }
            ResidualRef::None => panic!("expected gaussian"),
        }
    }

    #[test]
    fn evaluate_residuals_mae() {
        let p = EvalPolicy {
            window_n: None,
            min_quality: None,
            residual_ref: ResidualRef::None,
            ..policy()
        };
        let r = evaluate_residuals(&p, &[1.0, 2.0, 3.0], &[1.0, 2.0, 9.0], None);
        assert!(r.valid);
        // MAE = (0+0+6)/3 = 2 → normal (< 5)
        assert_eq!(r.flag, EvalFlag::Normal);
        assert!((r.score.unwrap() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn model_eval_from_predictions_keeps_fit() {
        let y = [1.0, 2.0, 3.0, 4.0];
        let yhat = [1.1, 1.9, 3.2, 3.8];
        let bundle = model_eval_from_predictions("lin", 1, &y, &yhat, 90.0, 99.0);
        assert!(bundle.fit.is_some());
        assert_eq!(bundle.fit.as_ref().unwrap().n, 4);
        let r = bundle.evaluate(0.0, 4, None);
        assert!(r.valid);
        assert_eq!(r.flag, EvalFlag::Normal);
    }

    #[test]
    fn flag_as_str() {
        assert_eq!(EvalFlag::Warning.as_str(), "warning");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip_policy_and_report() {
        let p = policy();
        let json = serde_json::to_string(&p).unwrap();
        let back: EvalPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
        let r = p.evaluate(6.0, 10, Some(0.9));
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"flag\":\"warning\""));
        assert!(json.contains("\"model_id\":\"hr_rest_v1\""));
        let back: EvalReport = serde_json::from_str(&json).unwrap();
        assert_eq!(r.flag, back.flag);
        assert_eq!(r.valid, back.valid);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_model_eval_omits_empty_fit() {
        let ev = ModelEval::from_policy(policy());
        let json = serde_json::to_string(&ev).unwrap();
        assert!(!json.contains("\"fit\""));
    }
}
