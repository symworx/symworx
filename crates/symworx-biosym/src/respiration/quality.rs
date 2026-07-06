// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

use crate::common::{BandpassParams, PeakDetectionParams, PhysiologyProcessingParams};

/// Respiration signal quality presets for analysis tuning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespSignalQuality {
    Reference,
    High,
    Moderate,
    Poor,
}

/// Legacy respiration bandpass: 0.1–0.5 Hz.
pub fn resp_default_bandpass() -> BandpassParams {
    BandpassParams {
        lowcut_hz: 0.1,
        highcut_hz: 0.5,
        q: 0.707,
        stages: 2,
    }
}

const RESP_FILTERED_MIN_HEIGHT: f64 = 0.02;

/// Reference / high quality respiration analysis.
pub fn resp_processing_reference() -> PhysiologyProcessingParams {
    PhysiologyProcessingParams {
        bandpass: Some(resp_default_bandpass()),
        peaks: PeakDetectionParams {
            min_height: Some(RESP_FILTERED_MIN_HEIGHT),
            ..Default::default()
        },
    }
}

/// Moderate noise respiration analysis.
pub fn resp_processing_moderate() -> PhysiologyProcessingParams {
    PhysiologyProcessingParams {
        bandpass: Some(resp_default_bandpass()),
        peaks: PeakDetectionParams {
            min_prominence: Some(0.08),
            ..Default::default()
        },
    }
}

/// Poor quality respiration analysis.
pub fn resp_processing_poor() -> PhysiologyProcessingParams {
    PhysiologyProcessingParams {
        bandpass: Some(resp_default_bandpass()),
        peaks: PeakDetectionParams {
            min_prominence: Some(0.12),
            min_height: Some(0.08),
            min_interval_sec: Some(2.5),
        },
    }
}

/// Map quality preset to processing parameters.
pub fn resp_processing_for_quality(quality: RespSignalQuality) -> PhysiologyProcessingParams {
    match quality {
        RespSignalQuality::Reference | RespSignalQuality::High => resp_processing_reference(),
        RespSignalQuality::Moderate => resp_processing_moderate(),
        RespSignalQuality::Poor => resp_processing_poor(),
    }
}

impl RespSignalQuality {
    /// Recommended bandpass + peak-detection settings for analysis.
    pub fn analysis_processing(&self) -> PhysiologyProcessingParams {
        resp_processing_for_quality(*self)
    }
}
