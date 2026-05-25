// Copyright (C) 2026 cSYMd, All rights reserved.

use symworx_core::processing::traits::PeakDetect;
use symworx_core::processing::PeakFinderBuilder;

/// Physiology data structure
pub struct PhysData {
    pub samples: Vec<f64>,
    pub timestamps: Vec<i64>,
}

/// Implementation for PhysData peak detection
impl PhysData {
    /// PPG-optimized peak finder with sensible defaults
    pub fn ppg_peaks(&self) -> PeakFinderBuilder {
        self.peaks()
            .prominence(0.12)
            .distance(15)        // ~150ms at 100Hz sampling
            .height(0.3)
    }

    /// Respiration-optimized peak finder
    pub fn resp_peaks(&self) -> PeakFinderBuilder {
        self.peaks()
            .prominence(0.08)
            .distance(35)        // slower breathing
            .height(0.1)
    }

    /// Generic access
    pub fn peaks(&self) -> PeakFinderBuilder {
        self.samples.peaks()
    }
}

/// PPG data metrics strucutre
pub struct PPGMetrics {
    pub hr: f32,
    pub rr_intervals: Vec<f32>,
    pub signal_quality: f32,
}

/// Respiratory data metrics structure
pub struct RespMetrics {
    pub resp_rate: f32,
    pub resp_intervals: Vec<f32>,
    pub signal_quality: f32,
}
