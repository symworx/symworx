// biosym/src/physiology/analysis/quality.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use crate::physiology::ppg::PPGNoiseConfig;


/// Define a `PPGSignalQuality` enum to set default
///  quality params (with flexibility for custom settings)
/// * This allows for easy iteration and looping through
///   various parameters to evaluate signal processing
///   and/or feature detection algorithms
#[derive(Debug, Clone)]
pub enum PPGSignalQuality {
    Reference,
    High,
    Moderate,
    Poor,
    Custom(PPGNoiseConfig),
}

impl From<PPGSignalQuality> for PPGNoiseConfig {
    fn from(q: PPGSignalQuality) -> Self {
        match q {
            PPGSignalQuality::Reference => PPGNoiseConfig {
                amp_drift_std   : 0.0,
                mu_drift_std    : 0.0,
                sigma_drift_std : 0.0,
                onset_jitter_std: 0.0,
                global_noise_std: 0.0,
                smoothing_kernel: 5,
            },
            PPGSignalQuality::High => PPGNoiseConfig {
                amp_drift_std   : 0.02,
                mu_drift_std    : 0.003,
                sigma_drift_std : 0.002,
                onset_jitter_std: 0.002,
                global_noise_std: 0.01,
                smoothing_kernel: 5,
            },
            PPGSignalQuality::Moderate => PPGNoiseConfig {
                amp_drift_std   : 0.08,
                mu_drift_std    : 0.01,
                sigma_drift_std : 0.006,
                onset_jitter_std: 0.006,
                global_noise_std: 0.04,
                smoothing_kernel: 5,
            },
            PPGSignalQuality::Poor => PPGNoiseConfig {
                amp_drift_std   : 0.20,
                mu_drift_std    : 0.02,
                sigma_drift_std : 0.015,
                onset_jitter_std: 0.015,
                global_noise_std: 0.12,
                smoothing_kernel: 0,
            },
            PPGSignalQuality::Custom(cfg) => cfg
        }
    }
}
