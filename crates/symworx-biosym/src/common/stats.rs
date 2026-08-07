// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

use symworx_core::{
    mean,
    std_dev,
};

use super::signal::PhysiologySignal;

/// Descriptive statistics for a physiological recording.
#[derive(Debug, Clone, PartialEq)]
pub struct PhysiologySummary {
    pub mean: f64,
    pub std_dev: f64,
    pub duration_sec: f64,
}

/// Compute mean, standard deviation, and duration for a signal.
pub fn summarize_signal(signal: &PhysiologySignal) -> PhysiologySummary {
    PhysiologySummary {
        mean: mean(&signal.samples),
        std_dev: std_dev(&signal.samples),
        duration_sec: signal.duration_sec(),
    }
}
