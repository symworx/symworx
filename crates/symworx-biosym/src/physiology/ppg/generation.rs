// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

use rand::rng;
use symworx_math::random::sample;

use super::PPGNoiseConfig;

// PPG Generation

/// Represents a PPG time-series signal.
#[derive(Debug, Clone)]
pub struct PPGTimeSeries {
    /// Times associated with PPG time series
    pub times: Vec<f64>,
    /// PPG readings
    pub values: Vec<f64>,
    /// Systolic peaks from PPG
    pub systolic_peaks: Vec<usize>,
    /// Diastolic peaks from PPG
    pub diastolic_peaks: Vec<usize>,
}

/// High-level parameters for PPG simulation (for consistency with respiration)
#[derive(Debug, Clone)]
pub struct PPGSimulationParams {
    /// Sampling frequency
    pub fs: f64,
    /// Duration of simulation
    pub duration: f64,
    /// Parameters used to simulate signal
    pub beat_params: (f64, f64, f64, f64, f64, f64), // (amp_s, mu_s, sigma_s, amp_d, mu_d, sigma_d)
    /// Noise parameter/configuration for simulation
    pub noise_config: PPGNoiseConfig,
    /// Seed the simulation for reproducibility
    pub seed: Option<u64>,
}

impl Default for PPGSimulationParams {
    fn default() -> Self {
        Self {
            fs: 250.0,
            duration: 10.0,
            beat_params: (1.0, 0.2, 0.03, 0.35, 0.45, 0.06),
            noise_config: PPGNoiseConfig::default(),
            seed: None,
        }
    }
}

/// Generate a single PPG beat using two Gaussians (systolic + diastolic).
pub fn generate_ppg_waveform(
    t0: f64,
    duration: f64,
    fs: f64,
    params: (f64, f64, f64, f64, f64, f64), // (amp_s, mu_s, sigma_s, amp_d, mu_d, sigma_d)
) -> (Vec<f64>, Vec<f64>) {
    let dt = 1.0 / fs;
    let n = (duration * fs).round() as usize;

    let (amp_s, mu_s, sigma_s, amp_d, mu_d, sigma_d) = params;

    let mut times = Vec::with_capacity(n);
    let mut values = Vec::with_capacity(n);

    for i in 0..n {
        let t = t0 + i as f64 * dt;
        let rel = t - t0;

        let g =
            |amp: f64, mu: f64, sigma: f64, x: f64| amp * (-0.5 * ((x - mu) / sigma).powi(2)).exp();

        let val = g(amp_s, mu_s, sigma_s, rel) + g(amp_d, mu_d, sigma_d, rel);

        times.push(t);
        values.push(val);
    }

    (times, values)
}

/// Stitch multiple PPG waveforms into one contiguous time series.
pub fn generate_ppg_timeseries(
    start_time: f64,
    rr_intervals: &[f64],
    count: usize,
    beat_duration: f64,
    fs: f64,
    beat_params: (f64, f64, f64, f64, f64, f64),
    noise_cfg: &PPGNoiseConfig,
) -> PPGTimeSeries {
    let mut times = Vec::new();
    let mut values = Vec::new();
    let mut systolic_peaks = Vec::new();
    let mut diastolic_peaks = Vec::new();

    let mut current_t = start_time;
    let mut rng = rng();

    for i in 0..count {
        let jitter = if noise_cfg.onset_jitter_std > 0.0 {
            sample::normal(&mut rng, 0.0, noise_cfg.onset_jitter_std)
        } else {
            0.0
        };

        let onset = current_t + jitter;
        let (btimes, bvals) = generate_ppg_waveform(onset, beat_duration, fs, beat_params);

        let base_index = times.len();

        times.extend(btimes);
        values.extend(bvals.iter().copied());

        // Peak detection
        let seg_len = bvals.len();
        for j in 1..(seg_len - 1) {
            if bvals[j] > bvals[j - 1] && bvals[j] > bvals[j + 1] {
                let rel_t = (j as f64) / fs;
                let dist_s = (rel_t - beat_params.1).abs();
                let dist_d = (rel_t - beat_params.4).abs();
                let global_idx = base_index + j;

                if dist_s <= dist_d {
                    systolic_peaks.push(global_idx);
                } else {
                    diastolic_peaks.push(global_idx);
                }
            }
        }

        // Advance time
        current_t += if i < rr_intervals.len() {
            rr_intervals[i]
        } else {
            beat_duration
        };
    }

    PPGTimeSeries {
        times,
        values,
        systolic_peaks,
        diastolic_peaks,
    }
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_ppg_waveform() {
        let params = (1.0, 0.2, 0.03, 0.35, 0.45, 0.06);
        let fs = 250.0;
        let beat_duration = 0.9;
        let rr_intervals = vec![0.8; 10];

        let noise = PPGNoiseConfig {
            onset_jitter_std: 0.01,
            ..Default::default()
        };

        let ts = generate_ppg_timeseries(0.0, &rr_intervals, 10, beat_duration, fs, params, &noise);

        assert!(!ts.times.is_empty());
        assert!(!ts.values.is_empty());
        assert!(!ts.systolic_peaks.is_empty());
    }
}
