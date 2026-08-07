// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Simple threshold status for live vitals.

use crate::types::StreamSample;

/// Coarse clinical-style status for a vitals snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VitalsStatus {
    /// Within normal thresholds.
    Normal,
    /// Elevated / borderline.
    Warning,
    /// Outside safe thresholds.
    Critical,
}

impl VitalsStatus {
    /// Short label for UI.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Warning => "Warning",
            Self::Critical => "Critical",
        }
    }

    /// Terminal-friendly color name (not an ANSI code).
    pub fn color_name(self) -> &'static str {
        match self {
            Self::Normal => "green",
            Self::Warning => "yellow",
            Self::Critical => "red",
        }
    }
}

/// Thresholds used by [`analyze_vitals`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VitalsThresholds {
    /// HR above this is at least Warning.
    pub hr_warning_high: f64,
    /// HR above this is Critical.
    pub hr_critical_high: f64,
    /// HR below this is Critical.
    pub hr_critical_low: f64,
    /// SpO₂ below this is Warning.
    pub spo2_warning_low: f64,
    /// SpO₂ below this is Critical.
    pub spo2_critical_low: f64,
}

impl Default for VitalsThresholds {
    fn default() -> Self {
        // Default bands:
        // Critical: hr > 120 or hr < 50 or spo2 < 90
        // Warning:  hr > 100 or spo2 < 94
        Self {
            hr_warning_high: 100.0,
            hr_critical_high: 120.0,
            hr_critical_low: 50.0,
            spo2_warning_low: 94.0,
            spo2_critical_low: 90.0,
        }
    }
}

/// Analyze a sample using default thresholds.
pub fn analyze_vitals(sample: &StreamSample) -> VitalsStatus {
    analyze_vitals_with(sample, &VitalsThresholds::default())
}

/// Analyze a sample with explicit thresholds.
///
/// Missing HR or SpO₂ does not trigger that axis (treated as non-alarming).
pub fn analyze_vitals_with(sample: &StreamSample, thr: &VitalsThresholds) -> VitalsStatus {
    let hr = sample.heart_rate();
    let spo2 = sample.spo2;

    let mut status = VitalsStatus::Normal;

    if let Some(hr) = hr {
        if hr > thr.hr_critical_high || hr < thr.hr_critical_low {
            return VitalsStatus::Critical;
        }
        if hr > thr.hr_warning_high {
            status = VitalsStatus::Warning;
        }
    }

    if let Some(spo2) = spo2 {
        if spo2 < thr.spo2_critical_low {
            return VitalsStatus::Critical;
        }
        if spo2 < thr.spo2_warning_low {
            status = VitalsStatus::Warning;
        }
    }

    status
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_hr() {
        let s = StreamSample {
            bpm: Some(72.0),
            spo2: Some(98.0),
            ..Default::default()
        };
        assert_eq!(analyze_vitals(&s), VitalsStatus::Normal);
    }

    #[test]
    fn warning_hr() {
        let s = StreamSample {
            bpm: Some(110.0),
            ..Default::default()
        };
        assert_eq!(analyze_vitals(&s), VitalsStatus::Warning);
    }

    #[test]
    fn critical_low_spo2() {
        let s = StreamSample {
            bpm: Some(80.0),
            spo2: Some(88.0),
            ..Default::default()
        };
        assert_eq!(analyze_vitals(&s), VitalsStatus::Critical);
    }
}
