// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

/// Noise / variability configuration for stitching PPG beats.
///
/// Used by [`crate::ppg::generate_ppg_timeseries`]:
/// - `amp_drift_std` / `mu_drift_std` / `sigma_drift_std` — per-beat random walk of
///   systolic & diastolic Gaussian height, timing, and width
/// - `onset_jitter_std` — RR-onset timing jitter (seconds)
/// - `global_noise_std` — additive noise on the finished series
/// - `smoothing_kernel` — moving-average window (≥2) to soften joins
#[derive(Clone, Debug)]
pub struct PPGNoiseConfig {
    /// Relative amplitude drift std (multiplicative, per beat)
    pub amp_drift_std: f64,
    /// Peak-time drift std in seconds (systolic/diastolic μ)
    pub mu_drift_std: f64,
    /// Width drift std in seconds (systolic/diastolic σ)
    pub sigma_drift_std: f64,
    /// Beat onset jitter std in seconds
    pub onset_jitter_std: f64,
    /// Additive Gaussian noise on the final signal
    pub global_noise_std: f64,
    /// Moving-average window (0 or 1 = off)
    pub smoothing_kernel: usize,
}

/// Default = Reference quality (no drift, no noise, smoothing=5)
impl Default for PPGNoiseConfig {
    fn default() -> Self {
        Self {
            amp_drift_std: 0.0,
            mu_drift_std: 0.0,
            sigma_drift_std: 0.0,
            onset_jitter_std: 0.0,
            global_noise_std: 0.0,
            smoothing_kernel: 5,
        }
    }
}
