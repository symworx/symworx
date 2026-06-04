// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

use symworx_core::{
    PeakFinderBuilder,
    signal::BandpassFilter,
};

use super::signal::PhysiologySignal;

/// Bandpass filter settings (2nd-order Butterworth stages via symworx-signal).
#[derive(Debug, Clone, PartialEq)]
pub struct BandpassParams {
    pub lowcut_hz: f64,
    pub highcut_hz: f64,
    /// Quality factor (0.707 ≈ Butterworth).
    pub q: f64,
    /// Number of cascaded 2nd-order sections (2 ≈ legacy 4th-order).
    pub stages: u8,
}

impl BandpassParams {
    /// Validate cutoffs against Nyquist.
    pub fn validate(&self, fs: f64) -> Result<(), &'static str> {
        if self.lowcut_hz <= 0.0 || self.highcut_hz <= 0.0 {
            return Err("cutoff frequencies must be positive");
        }
        if self.lowcut_hz >= self.highcut_hz {
            return Err("lowcut must be less than highcut");
        }
        if self.highcut_hz >= fs / 2.0 {
            return Err("highcut must be below Nyquist frequency");
        }
        Ok(())
    }
}

/// Optional overrides for peak detection on top of domain presets.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PeakDetectionParams {
    pub min_height: Option<f64>,
    pub min_prominence: Option<f64>,
    pub min_interval_sec: Option<f64>,
}

/// Pre-peak processing pipeline configuration.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PhysiologyProcessingParams {
    pub bandpass: Option<BandpassParams>,
    pub peaks: PeakDetectionParams,
}

impl PhysiologyProcessingParams {
    /// No filtering; peak preset only.
    pub fn none() -> Self {
        Self::default()
    }
}

/// Apply cascaded bandpass filtering in place on a sample vector.
pub fn apply_bandpass(
    samples: &[f64],
    fs: f64,
    params: &BandpassParams,
) -> Result<Vec<f64>, &'static str> {
    params.validate(fs)?;
    let stages = params.stages.max(1);
    let mut out = samples.to_vec();
    for _ in 0..stages {
        let mut filter = BandpassFilter::new(fs, params.lowcut_hz, params.highcut_hz, params.q);
        out = filter.process(&out);
    }
    Ok(out)
}

/// Apply optional bandpass from processing params.
pub fn preprocess_signal(
    mut signal: PhysiologySignal,
    params: &PhysiologyProcessingParams,
) -> Result<PhysiologySignal, &'static str> {
    if let Some(ref bp) = params.bandpass {
        signal.samples = apply_bandpass(&signal.samples, signal.fs, bp)?;
    }
    Ok(signal)
}

/// Apply peak overrides onto a domain-specific finder builder.
pub fn apply_peak_overrides<'a>(
    finder: PeakFinderBuilder<'a>,
    fs: f64,
    base_min_interval_sec: f64,
    base_prominence: f64,
    base_height: f64,
    overrides: &PeakDetectionParams,
) -> PeakFinderBuilder<'a> {
    let min_interval = overrides.min_interval_sec.unwrap_or(base_min_interval_sec);
    let distance = ((min_interval * fs).round() as usize).max(1);
    let prominence = overrides.min_prominence.unwrap_or(base_prominence);
    let height = overrides.min_height.unwrap_or(base_height);

    finder
        .distance(distance)
        .prominence(prominence)
        .height(height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bandpass_attenuates_dc() {
        let fs = 100.0;
        let samples = vec![1.0; 200];
        let params = BandpassParams {
            lowcut_hz: 0.5,
            highcut_hz: 5.0,
            q: 0.707,
            stages: 2,
        };
        let filtered = apply_bandpass(&samples, fs, &params).unwrap();
        // Ignore startup transient; tail should be strongly attenuated.
        let tail_energy: f64 = filtered[100..].iter().map(|v| v.abs()).sum();
        assert!(
            tail_energy < 2.0,
            "DC tail should be attenuated, energy={tail_energy}"
        );
    }

    #[test]
    fn invalid_bandpass_rejected() {
        let params = BandpassParams {
            lowcut_hz: 10.0,
            highcut_hz: 5.0,
            q: 0.707,
            stages: 1,
        };
        assert!(apply_bandpass(&[0.0; 10], 100.0, &params).is_err());
    }
}
