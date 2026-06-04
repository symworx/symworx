// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

/// A uniformly sampled physiological waveform.
#[derive(Debug, Clone)]
pub struct PhysiologySignal {
    /// Sampling frequency (Hz).
    pub fs: f64,
    /// Amplitude samples.
    pub samples: Vec<f64>,
    /// Optional time axis (seconds). When absent, times are derived as `i / fs`.
    pub times: Option<Vec<f64>>,
}

impl PhysiologySignal {
    /// Create a signal from samples and sampling rate.
    pub fn new(fs: f64, samples: Vec<f64>) -> Self {
        Self {
            fs,
            samples,
            times: None,
        }
    }

    /// Create a signal with an explicit time axis; `fs` is inferred when possible.
    pub fn with_times(fs: f64, samples: Vec<f64>, times: Vec<f64>) -> Self {
        let fs = infer_fs(&times).unwrap_or(fs);
        Self {
            fs,
            samples,
            times: Some(times),
        }
    }

    /// Duration of the recording in seconds.
    pub fn duration_sec(&self) -> f64 {
        if self.fs <= 0.0 {
            return 0.0;
        }
        self.samples.len() as f64 / self.fs
    }
}

/// Infer sampling rate from a monotonic time vector.
pub fn infer_fs(times: &[f64]) -> Option<f64> {
    if times.len() < 2 {
        return None;
    }
    let dt = times[1] - times[0];
    if dt > 0.0 { Some(1.0 / dt) } else { None }
}
