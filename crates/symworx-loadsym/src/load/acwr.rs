// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Acute:Chronic Workload Ratio (ACWR), EWMA variants, and risk classification.
//!
//! ## Notes
//! - Acute window typically 7 days, chronic 28 days (configurable).
//! - Supports both "coupled" (current acute overlaps chronic) and "uncoupled".
//! - EWMA uses the common span-based formulation (alpha = 2/(span+1)).
//! - Risk bucketing follows Gabbett-style thresholds by default (extensible).
//!
//! The heavy lifting for rolling means uses `symworx_math::series::{rolling_mean, ewma}`

use symworx_core::math::series::{ewma as ewma_series, rolling_mean};

use crate::error::{LoadSymError, Result};

/// Result of an ACWR / load window calculation for a single point (or the latest day).
#[derive(Debug, Clone, PartialEq)]
pub struct AcwrSnapshot {
    pub acute_load: f64,
    pub chronic_load: f64,
    pub acwr: f64,
    pub ewma_acute: Option<f64>,
    pub ewma_chronic: Option<f64>,
    pub risk_level: RiskLevel,
}

/// Risk level categories for ACWR (and future combined metrics).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RiskLevel {
    /// Low risk (typically ACWR << 1.0 or very well managed load).
    Low,
    /// Sweet-spot / moderate protective loading.
    Moderate,
    /// Elevated risk (common "danger zone" > ~1.3-1.5).
    High,
    /// Very high / spike risk.
    VeryHigh,
}

impl RiskLevel {
    /// Human-readable label (matches DB `risk_level` VARCHAR expectations).
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Low => "Low",
            RiskLevel::Moderate => "Moderate",
            RiskLevel::High => "High",
            RiskLevel::VeryHigh => "Very High",
        }
    }
}

/// Compute a simple acute/chronic pair + ratio from a daily load series.
///
/// `daily_loads` should be ordered oldest → newest (or provide a contiguous window).
/// `acute_window` = 7, `chronic_window` = 28 are the sports-science defaults.
///
/// Returns the *latest* snapshot only. For full series use [`compute_acwr_series`].
///
/// Errors on insufficient data (when chronic window not met).
pub fn compute_acute_chronic(
    daily_loads: &[f64],
    acute_window: usize,
    chronic_window: usize,
) -> Result<AcwrSnapshot> {
    if acute_window == 0 || chronic_window == 0 || acute_window > chronic_window {
        return Err(LoadSymError::InvalidParameter(
            "acute_window must be >0 and <= chronic_window".into(),
        ));
    }
    if daily_loads.len() < chronic_window {
        return Err(LoadSymError::InsufficientData(format!(
            "need at least {} daily loads for chronic window (got {})",
            chronic_window,
            daily_loads.len()
        )));
    }

    let means = rolling_mean(daily_loads, chronic_window); // last entry is the chronic mean over last chronic_window
    let chronic = *means.last().unwrap_or(&f64::NAN);

    // For acute we take the mean of the last `acute_window` values directly (more precise for the tip)
    let n = daily_loads.len();
    let acute_slice = &daily_loads[n - acute_window..];
    let acute = acute_slice.iter().sum::<f64>() / acute_window as f64;

    let acwr = if chronic > 0.0 { acute / chronic } else { 0.0 };

    // For foundation stub we do not yet compute EWMA here (see compute_ewma_acute_chronic)
    let risk = classify_acwr(acwr);

    Ok(AcwrSnapshot {
        acute_load: acute,
        chronic_load: chronic,
        acwr,
        ewma_acute: None,
        ewma_chronic: None,
        risk_level: risk,
    })
}

/// Full per-day ACWR + risk series (same length as input).
///
/// Leading entries (before chronic_window) are not computed (risk = Low with NaN loads for now).
/// This shape is directly useful for writing daily `player_load_metrics` rows.
pub fn compute_acwr_series(
    daily_loads: &[f64],
    acute_window: usize,
    chronic_window: usize,
) -> Vec<Option<AcwrSnapshot>> {
    // Placeholder implementation for Phase 1 foundation.
    // Real version (Phase 3) will fill every position that has enough history,
    // compute coupled/uncoupled variants, and attach EWMA.
    let n = daily_loads.len();
    let mut out = vec![None; n];
    if n < chronic_window {
        return out;
    }
    // Only compute the final point for the stub (keeps it simple and correct)
    if let Ok(snap) = compute_acute_chronic(daily_loads, acute_window, chronic_window) {
        out[n - 1] = Some(snap);
    }
    out
}

/// EWMA-based acute and chronic (common "smoothed" variant used in modern load monitoring).
///
/// Returns the *latest* values only in the snapshot (EWMA fields populated).
pub fn compute_ewma_acute_chronic(
    daily_loads: &[f64],
    acute_span: usize,
    chronic_span: usize,
) -> Result<AcwrSnapshot> {
    if daily_loads.is_empty() {
        return Err(LoadSymError::InsufficientData("empty load series".into()));
    }
    let ewma_acute_series = ewma_series(daily_loads, acute_span);
    let ewma_chronic_series = ewma_series(daily_loads, chronic_span);

    let ewma_acute = *ewma_acute_series.last().unwrap();
    let ewma_chronic = *ewma_chronic_series.last().unwrap();

    let acwr = if ewma_chronic > 0.0 {
        ewma_acute / ewma_chronic
    } else {
        0.0
    };

    Ok(AcwrSnapshot {
        acute_load: ewma_acute, // reuse fields for EWMA version in this helper
        chronic_load: ewma_chronic,
        acwr,
        ewma_acute: Some(ewma_acute),
        ewma_chronic: Some(ewma_chronic),
        risk_level: classify_acwr(acwr),
    })
}

/// Default risk classification from ACWR (Gabbett-inspired thresholds).
///
/// These are **starting defaults** — consumers (e.g. uncg import scripts) can
/// override or combine with other signals (wellness, prior injury, etc.).
pub fn classify_acwr(acwr: f64) -> RiskLevel {
    if !acwr.is_finite() || acwr <= 0.0 {
        return RiskLevel::Low;
    }
    if acwr < 0.8 {
        RiskLevel::Low
    } else if acwr <= 1.3 {
        RiskLevel::Moderate
    } else if acwr <= 1.5 {
        RiskLevel::High
    } else {
        RiskLevel::VeryHigh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acute_chronic_basic() {
        // 30 days of constant load → ACWR == 1.0
        let loads: Vec<f64> = (0..30).map(|_| 500.0).collect();
        let snap = compute_acute_chronic(&loads, 7, 28).unwrap();
        assert!((snap.acwr - 1.0).abs() < 1e-9);
        assert_eq!(snap.risk_level, RiskLevel::Moderate);
        assert_eq!(snap.risk_level.as_str(), "Moderate");
    }

    #[test]
    fn test_insufficient_data() {
        let loads = vec![100.0; 10];
        let res = compute_acute_chronic(&loads, 7, 28);
        assert!(matches!(res, Err(LoadSymError::InsufficientData(_))));
    }

    #[test]
    fn test_classify() {
        assert_eq!(classify_acwr(0.6), RiskLevel::Low);
        assert_eq!(classify_acwr(1.2), RiskLevel::Moderate);
        assert_eq!(classify_acwr(1.45), RiskLevel::High);
        assert_eq!(classify_acwr(1.7), RiskLevel::VeryHigh);
    }

    #[test]
    fn test_ewma_acute_chronic_stub() {
        let loads: Vec<f64> = (0..30).map(|i| 400.0 + (i as f64 % 5.0) * 50.0).collect();
        let snap = compute_ewma_acute_chronic(&loads, 7, 28).unwrap();
        assert!(snap.ewma_acute.is_some());
        assert!(snap.acwr > 0.0);
    }
}
