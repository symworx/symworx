// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.


/// Noise Configuration for stitching beats
#[derive(Clone, Debug)]
pub struct PPGNoiseConfig {
    pub amp_drift_std    : f64,   // relative drift
    pub mu_drift_std     : f64,   // seconds
    pub sigma_drift_std  : f64,   // seconds
    pub onset_jitter_std : f64,   // 
    pub global_noise_std : f64,   // noise on final signal
    pub smoothing_kernel : usize, // moving average window
}

/// Default = Reference quality (no drift, no noise, smoothing=5)
impl Default for PPGNoiseConfig {
    fn default() -> Self {
        Self {
            amp_drift_std    : 0.0,
            mu_drift_std     : 0.0,
            sigma_drift_std  : 0.0,
            onset_jitter_std : 0.0,
            global_noise_std : 0.0,
            smoothing_kernel : 5,
        }
    }
}
