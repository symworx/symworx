// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

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

#[inline]
fn gaussian(amp: f64, mu: f64, sigma: f64, x: f64) -> f64 {
    if sigma <= 1e-12 {
        return 0.0;
    }
    amp * (-0.5 * ((x - mu) / sigma).powi(2)).exp()
}

/// Keep systolic / diastolic Gaussians physiologically ordered and positive.
fn clamp_beat_params(params: (f64, f64, f64, f64, f64, f64), beat_duration: f64) -> (f64, f64, f64, f64, f64, f64) {
    let (mut amp_s, mut mu_s, mut sigma_s, mut amp_d, mut mu_d, mut sigma_d) = params;
    let bd = beat_duration.max(0.4);

    amp_s = amp_s.clamp(0.35, 1.8);
    amp_d = amp_d.clamp(0.08, 0.9);
    sigma_s = sigma_s.clamp(0.012, 0.07);
    sigma_d = sigma_d.clamp(0.025, 0.12);
    mu_s = mu_s.clamp(0.08, bd * 0.4);
    // Diastolic notch/peak after systolic, with room for the tail
    let mu_d_min = (mu_s + 0.1).min(bd * 0.75);
    mu_d = mu_d.clamp(mu_d_min, (bd * 0.85).max(mu_d_min + 0.02));

    (amp_s, mu_s, sigma_s, amp_d, mu_d, sigma_d)
}

/// Random-walk the systolic/diastolic Gaussian parameters between beats.
fn drift_beat_params(
    params: (f64, f64, f64, f64, f64, f64),
    noise_cfg: &PPGNoiseConfig,
    beat_duration: f64,
    rng: &mut impl rand::Rng,
) -> (f64, f64, f64, f64, f64, f64) {
    let (mut amp_s, mut mu_s, mut sigma_s, mut amp_d, mut mu_d, mut sigma_d) = params;

    if noise_cfg.amp_drift_std > 0.0 {
        // Relative height jitter (both waves; diastolic can move a bit more independently)
        let ds = sample::normal(rng, 0.0, noise_cfg.amp_drift_std);
        let dd = sample::normal(rng, 0.0, noise_cfg.amp_drift_std * 1.15);
        amp_s *= 1.0 + ds;
        amp_d *= 1.0 + dd;
    }
    if noise_cfg.mu_drift_std > 0.0 {
        mu_s += sample::normal(rng, 0.0, noise_cfg.mu_drift_std);
        mu_d += sample::normal(rng, 0.0, noise_cfg.mu_drift_std * 1.1);
    }
    if noise_cfg.sigma_drift_std > 0.0 {
        sigma_s += sample::normal(rng, 0.0, noise_cfg.sigma_drift_std);
        sigma_d += sample::normal(rng, 0.0, noise_cfg.sigma_drift_std * 1.1);
    }

    clamp_beat_params((amp_s, mu_s, sigma_s, amp_d, mu_d, sigma_d), beat_duration)
}

fn moving_average(data: &[f64], kernel: usize) -> Vec<f64> {
    if kernel <= 1 || data.is_empty() {
        return data.to_vec();
    }
    let half = kernel / 2;
    let mut out = Vec::with_capacity(data.len());
    for i in 0..data.len() {
        let start = i.saturating_sub(half);
        let end = (i + half + 1).min(data.len());
        let sum: f64 = data[start..end].iter().sum();
        out.push(sum / (end - start) as f64);
    }
    out
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
    let params = clamp_beat_params(params, duration);
    let (amp_s, mu_s, sigma_s, amp_d, mu_d, sigma_d) = params;

    let mut times = Vec::with_capacity(n);
    let mut values = Vec::with_capacity(n);

    for i in 0..n {
        let t = t0 + i as f64 * dt;
        let rel = t - t0;
        let val = gaussian(amp_s, mu_s, sigma_s, rel) + gaussian(amp_d, mu_d, sigma_d, rel);
        times.push(t);
        values.push(val);
    }

    (times, values)
}

/// Stitch multiple PPG waveforms into one contiguous time series.
///
/// Beats are **overlap-added** onto a uniform time grid so RR spacing is
/// continuous (no sample-stream discontinuities / reversed timestamps).
///
/// Per-beat systolic/diastolic height and shape vary via `noise_cfg` drifts
/// (`amp_drift_std`, `mu_drift_std`, `sigma_drift_std`). Onset jitter, additive
/// global noise, and optional smoothing are also applied.
pub fn generate_ppg_timeseries(
    start_time: f64,
    rr_intervals: &[f64],
    count: usize,
    beat_duration: f64,
    fs: f64,
    beat_params: (f64, f64, f64, f64, f64, f64),
    noise_cfg: &PPGNoiseConfig,
) -> PPGTimeSeries {
    let n_beats = count.max(1);
    let dt = 1.0 / fs;
    let beat_duration = beat_duration.max(0.4);
    let mut rng = rng();

    // Beat onsets (RR schedule + optional onset jitter)
    let mut onsets = Vec::with_capacity(n_beats);
    let mut current_t = start_time;
    for i in 0..n_beats {
        let jitter = if noise_cfg.onset_jitter_std > 0.0 {
            sample::normal(&mut rng, 0.0, noise_cfg.onset_jitter_std)
        } else {
            0.0
        };
        onsets.push((current_t + jitter).max(start_time));
        let rr = if i < rr_intervals.len() {
            rr_intervals[i].max(0.3)
        } else {
            beat_duration
        };
        current_t += rr;
    }

    // Uniform grid covering all onsets + diastolic tail of the last beat
    let end_time = current_t + beat_duration * 0.35;
    let n = ((end_time - start_time) * fs).ceil().max(1.0) as usize;
    let times: Vec<f64> = (0..n).map(|i| start_time + i as f64 * dt).collect();
    let mut values = vec![0.0; n];

    let mut params = clamp_beat_params(beat_params, beat_duration);
    let mut systolic_peaks = Vec::with_capacity(n_beats);
    let mut diastolic_peaks = Vec::with_capacity(n_beats);

    let n_beat_samples = (beat_duration * fs).round().max(1.0) as usize;

    for &onset in &onsets {
        // Independent shape/height walk each beat (when drift stds > 0)
        params = drift_beat_params(params, noise_cfg, beat_duration, &mut rng);
        let (amp_s, mu_s, sigma_s, amp_d, mu_d, sigma_d) = params;

        let start_idx = ((onset - start_time) * fs).round() as isize;
        let mut best_s = (0usize, f64::NEG_INFINITY);
        let mut best_d = (0usize, f64::NEG_INFINITY);

        for j in 0..n_beat_samples {
            let idx = start_idx + j as isize;
            if idx < 0 {
                continue;
            }
            let idx = idx as usize;
            if idx >= n {
                break;
            }
            let rel = j as f64 * dt;
            let val = gaussian(amp_s, mu_s, sigma_s, rel) + gaussian(amp_d, mu_d, sigma_d, rel);
            values[idx] += val;

            // Track local max near expected systolic / diastolic centers
            if (rel - mu_s).abs() <= (3.0 * sigma_s).max(0.04) && values[idx] > best_s.1 {
                best_s = (idx, values[idx]);
            }
            if (rel - mu_d).abs() <= (3.0 * sigma_d).max(0.06) && values[idx] > best_d.1 {
                best_d = (idx, values[idx]);
            }
        }

        if best_s.1.is_finite() && best_s.1 > 0.0 {
            systolic_peaks.push(best_s.0);
        }
        if best_d.1.is_finite() && best_d.1 > 0.0 {
            diastolic_peaks.push(best_d.0);
        }
    }

    // Additive sensor / measurement noise
    if noise_cfg.global_noise_std > 0.0 {
        for v in values.iter_mut() {
            *v += sample::normal(&mut rng, 0.0, noise_cfg.global_noise_std);
        }
    }

    // Light smoothing for cleaner joins / less grain
    if noise_cfg.smoothing_kernel > 1 {
        values = moving_average(&values, noise_cfg.smoothing_kernel);
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
    fn generate_ppg_waveform_basic() {
        let params = (1.0, 0.2, 0.03, 0.35, 0.45, 0.06);
        let (times, values) = generate_ppg_waveform(0.0, 0.9, 250.0, params);
        assert_eq!(times.len(), values.len());
        assert!(!values.is_empty());
        // Peak should be near systolic amp
        let max_v = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(max_v > 0.5);
    }

    #[test]
    fn generate_ppg_timeseries_contiguous() {
        let params = (1.0, 0.2, 0.03, 0.35, 0.45, 0.06);
        let fs = 250.0;
        let beat_duration = 0.9;
        let rr_intervals = vec![0.8; 10];

        let noise = PPGNoiseConfig {
            amp_drift_std: 0.05,
            mu_drift_std: 0.005,
            sigma_drift_std: 0.003,
            onset_jitter_std: 0.01,
            global_noise_std: 0.01,
            smoothing_kernel: 5,
        };

        let ts = generate_ppg_timeseries(0.0, &rr_intervals, 10, beat_duration, fs, params, &noise);

        assert!(!ts.times.is_empty());
        assert_eq!(ts.times.len(), ts.values.len());
        assert!(!ts.systolic_peaks.is_empty());

        // Strictly increasing uniform grid
        for w in ts.times.windows(2) {
            assert!(w[1] > w[0], "time grid must be strictly increasing");
            assert!((w[1] - w[0] - 1.0 / fs).abs() < 1e-9);
        }
    }

    #[test]
    fn generate_ppg_beat_variation() {
        // With large amp drift, successive runs should not be identical
        let params = (1.0, 0.2, 0.03, 0.35, 0.45, 0.06);
        let noise = PPGNoiseConfig {
            amp_drift_std: 0.12,
            mu_drift_std: 0.01,
            sigma_drift_std: 0.008,
            onset_jitter_std: 0.0,
            global_noise_std: 0.0,
            smoothing_kernel: 0,
        };
        let rr = vec![0.85; 8];
        let a = generate_ppg_timeseries(0.0, &rr, 8, 0.9, 100.0, params, &noise);
        let b = generate_ppg_timeseries(0.0, &rr, 8, 0.9, 100.0, params, &noise);
        assert_eq!(a.values.len(), b.values.len());
        let max_diff = a
            .values
            .iter()
            .zip(b.values.iter())
            .map(|(x, y)| (x - y).abs())
            .fold(0.0_f64, f64::max);
        assert!(
            max_diff > 1e-3,
            "expected beat-to-beat / run variation, max_diff={}",
            max_diff
        );
    }
}
