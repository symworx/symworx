// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

use crate::common::processing::{
    BandpassParams,
    PeakDetectionParams,
    PhysiologyProcessingParams,
};

/// Default bandpass for gait marker/angle signals (e.g. pelvis vertical or joint angles).
/// Broad enough to pass step frequencies (~0.5-3 Hz) while attenuating DC and high-freq noise.
pub fn gait_default_bandpass() -> BandpassParams {
    BandpassParams {
        lowcut_hz: 0.2,
        highcut_hz: 6.0,
        q: 0.707,
        stages: 2,
    }
}

/// Post-bandpass min height for typical simulated gait signals (normalized-ish amplitudes).
const GAIT_FILTERED_MIN_HEIGHT: f64 = 0.05;

/// Reference / high-quality gait event detection (mild bandpass + basic height).
pub fn gait_processing_reference() -> PhysiologyProcessingParams {
    PhysiologyProcessingParams {
        bandpass: Some(gait_default_bandpass()),
        peaks: PeakDetectionParams {
            min_height: Some(GAIT_FILTERED_MIN_HEIGHT),
            ..Default::default()
        },
    }
}

/// High-quality recordings: same as reference for gait.
pub fn gait_processing_high() -> PhysiologyProcessingParams {
    gait_processing_reference()
}

/// Moderate noise: add prominence to suppress spurious peaks.
pub fn gait_processing_moderate() -> PhysiologyProcessingParams {
    PhysiologyProcessingParams {
        bandpass: Some(gait_default_bandpass()),
        peaks: PeakDetectionParams {
            min_prominence: Some(0.10),
            ..Default::default()
        },
    }
}

/// Poor quality: stricter criteria.
pub fn gait_processing_poor() -> PhysiologyProcessingParams {
    PhysiologyProcessingParams {
        bandpass: Some(gait_default_bandpass()),
        peaks: PeakDetectionParams {
            min_prominence: Some(0.15),
            min_height: Some(0.20),
            min_interval_sec: Some(0.6),
        },
    }
}

/// Map quality to processing params for gait stride detection.
pub fn gait_processing_for_quality(quality: super::quality::GaitSignalQuality) -> PhysiologyProcessingParams {
    match quality {
        super::quality::GaitSignalQuality::Reference | super::quality::GaitSignalQuality::High => {
            gait_processing_high()
        }
        super::quality::GaitSignalQuality::Moderate => gait_processing_moderate(),
        super::quality::GaitSignalQuality::Poor => gait_processing_poor(),
    }
}
