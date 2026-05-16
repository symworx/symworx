// biosym/src/physiology/models.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

/// Physiology data structure
pub struct physData {
    pub samples: Vec<f64>,
    pub timestamps: Vec<i64>,
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

