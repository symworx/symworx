// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

use crate::physiology::{
    common::{
        BandpassParams,
        PeakDetectionParams,
        PhysiologyProcessingParams,
    },
    ppg::PPGSignalQuality,
};

/// Legacy PPG bandpass: 0.5–5.0 Hz (4th-order equivalent via 2 cascaded 2nd-order sections).
pub fn ppg_default_bandpass() -> BandpassParams {
    BandpassParams {
        lowcut_hz: 0.5,
        highcut_hz: 5.0,
        q: 0.707,
        stages: 2,
    }
}

/// Post-bandpass peak height (filtered amplitudes are smaller than raw).
const PPG_FILTERED_MIN_HEIGHT: f64 = 0.05;

/// Processing pipeline tuned for simulated/reference PPG.
pub fn ppg_processing_reference() -> PhysiologyProcessingParams {
    PhysiologyProcessingParams {
        bandpass: Some(ppg_default_bandpass()),
        peaks: PeakDetectionParams {
            min_height: Some(PPG_FILTERED_MIN_HEIGHT),
            ..Default::default()
        },
    }
}

/// High-quality recordings: bandpass with default peak preset.
pub fn ppg_processing_high() -> PhysiologyProcessingParams {
    ppg_processing_reference()
}

/// Moderate noise: stronger prominence threshold.
pub fn ppg_processing_moderate() -> PhysiologyProcessingParams {
    PhysiologyProcessingParams {
        bandpass: Some(ppg_default_bandpass()),
        peaks: PeakDetectionParams {
            min_prominence: Some(0.15),
            ..Default::default()
        },
    }
}

/// Poor quality: bandpass + stricter peak criteria.
pub fn ppg_processing_poor() -> PhysiologyProcessingParams {
    PhysiologyProcessingParams {
        bandpass: Some(ppg_default_bandpass()),
        peaks: PeakDetectionParams {
            min_prominence: Some(0.20),
            min_height: Some(0.35),
            min_interval_sec: Some(0.45),
        },
    }
}

/// Map [`PPGSignalQuality`] to recommended analysis processing (noise config is separate).
pub fn ppg_processing_for_quality(quality: PPGSignalQuality) -> PhysiologyProcessingParams {
    match quality {
        PPGSignalQuality::Reference | PPGSignalQuality::High => ppg_processing_high(),
        PPGSignalQuality::Moderate => ppg_processing_moderate(),
        PPGSignalQuality::Poor => ppg_processing_poor(),
        PPGSignalQuality::Custom(_) => ppg_processing_moderate(),
    }
}
