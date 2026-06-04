// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Physiology-specific processing (re-exports the generic processing primitives
//! from `crate::common::processing` for backward compatibility + signal-aware preprocess).

use super::signal::PhysiologySignal;
pub use crate::common::processing::{
    BandpassParams,
    PeakDetectionParams,
    PhysiologyProcessingParams,
    apply_bandpass,
    apply_peak_overrides,
};

/// Apply optional bandpass from processing params (physiology-signal specific).
pub fn preprocess_signal(
    mut signal: PhysiologySignal,
    params: &PhysiologyProcessingParams,
) -> Result<PhysiologySignal, &'static str> {
    if let Some(ref bp) = params.bandpass {
        signal.samples = apply_bandpass(&signal.samples, signal.fs, bp)?;
    }
    Ok(signal)
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
