// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

use crate::common::processing::PhysiologyProcessingParams;

/// Gait signal quality presets for analysis (stride event detection) tuning.
/// Mirrors the structure used for PPG and respiration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaitSignalQuality {
    Reference,
    High,
    Moderate,
    Poor,
}

impl GaitSignalQuality {
    /// Recommended bandpass + peak-detection settings for gait stride analysis.
    pub fn analysis_processing(&self) -> PhysiologyProcessingParams {
        super::processing::gait_processing_for_quality(*self)
    }
}
