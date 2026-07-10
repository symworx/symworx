// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

use rand::{Rng, SeedableRng, rngs::StdRng};

/// Represents a respiration time-series signal.
#[derive(Debug, Clone)]
pub struct RespTimeSeries {
    /// Sample times (seconds)
    pub times: Vec<f64>,
    /// Respiratory flow (approx dV/dt)
    pub flow: Vec<f64>,
    /// Respiratory volume (tidal, closed-cycle)
    pub volume: Vec<f64>,
    /// Peak inhalation (volume maxima)
    pub inhalation_peaks: Vec<usize>,
    /// Peak exhalation (volume minima / end-exp)
    pub exhalation_peaks: Vec<usize>,
}

/// Respiration simulation parameters (gamma-shaped inspiration + exponential expiration)
#[derive(Debug, Clone)]
pub struct RespSimulationParams {
    /// Breaths per minute
    pub brpm: f64,
    /// Duration (min)
    pub dur_min: f64,
    /// Sampling frequency (Hz)
    pub fs: f64,
    /// Tidal volume (volume units)
    pub tidal_volume: f64,
    /// Inspiration / expiration duration ratio (I:E as insp/exp)
    pub insp_exp_ratio: f64,
    /// Inspiration shape parameter (higher → sharper rise late in insp)
    pub kappa_insp: f64,
    /// Expiration time-constant scale (relative to exp duration; larger → slower empty)
    pub tau_exp: f64,
    /// Amplitude scale (multiplies tidal volume)
    pub amplitude: f64,
    /// Zero-mean noise on volume (fraction of tidal volume)
    pub noise_level: f64,
    /// Seed for reproducibility
    pub seed: Option<u64>,
}

impl Default for RespSimulationParams {
    fn default() -> Self {
        Self {
            brpm: 12.0,
            dur_min: 1.0,
            fs: 50.0,
            tidal_volume: 0.5,
            insp_exp_ratio: 0.5,
            kappa_insp: 4.5,
            tau_exp: 2.8,
            amplitude: 1.0,
            noise_level: 0.0,
            seed: None,
        }
    }
}

/// Smoothstep 0→1 for a unit phase u ∈ [0, 1].
#[inline]
fn smooth01(u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// One closed breath on a unit phase φ ∈ [0, 1):
/// - inspiration [0, f_insp): volume 0 → V_tidal (gamma-like rise)
/// - expiration  [f_insp, 1): volume V_tidal → 0 (exponential-like empty)
///
/// Guarantees V(0) = V(1) = 0 so tiling cycles cannot accumulate baseline.
fn breath_volume_at_phase(
    phase: f64,
    f_insp: f64,
    kappa_insp: f64,
    tau_exp: f64,
    tidal: f64,
) -> f64 {
    let phase = phase.rem_euclid(1.0);
    let f_insp = f_insp.clamp(0.15, 0.7);
    let tidal = tidal.max(0.0);

    if phase < f_insp {
        // Inspiration: power/gamma-like cumulative rise 0 → 1
        let u = (phase / f_insp).clamp(0.0, 1.0);
        // (1 - (1-u)^κ) rises faster late for κ > 1; smooth ends
        let shape = 1.0 - (1.0 - u).powf(kappa_insp.max(1.0));
        // Blend with smoothstep for C1-ish endpoints
        let rise = 0.65 * shape + 0.35 * smooth01(u);
        tidal * rise
    } else {
        // Expiration: start at tidal, decay to 0 by phase=1
        let u = ((phase - f_insp) / (1.0 - f_insp)).clamp(0.0, 1.0);
        // Map tau_exp to a dimensionless rate so V(1) ≈ 0
        // Higher tau_exp → slower early decay, steeper finish via complement smooth
        let rate = (2.2 + 0.35 * tau_exp.max(0.5)).max(1.5);
        let exp_dec = (-rate * u).exp();
        // Force exact return to 0 at u=1
        let dec = (exp_dec - (-rate).exp()) / (1.0 - (-rate).exp()).max(1e-9);
        let fall = 0.7 * dec + 0.3 * (1.0 - smooth01(u));
        tidal * fall.clamp(0.0, 1.0)
    }
}

/// Generate a respiration timeseries.
///
/// Volume is synthesized as **closed inhale→exhale cycles** (return-to-baseline
/// each breath), then tiled. Flow is the numerical derivative of volume.
/// This keeps the series relatively stationary without low-frequency AC drift.
pub fn generate_respiration_timeseries(params: &RespSimulationParams) -> RespTimeSeries {
    let total_duration = params.dur_min * 60.0;
    let n_samples = (total_duration * params.fs).round().max(1.0) as usize;
    let dt = 1.0 / params.fs;

    let cycle_time = 60.0 / params.brpm.max(1.0);
    // I:E = insp_exp_ratio means insp = ratio * exp, so
    // f_insp = insp / cycle = ratio / (1 + ratio)
    let f_insp =
        (params.insp_exp_ratio / (1.0 + params.insp_exp_ratio.max(1e-6))).clamp(0.15, 0.7);
    let tidal = (params.tidal_volume * params.amplitude).max(1e-9);

    let mut rng = match params.seed {
        Some(s) => StdRng::seed_from_u64(s),
        None => StdRng::from_os_rng(),
    };

    let mut times = Vec::with_capacity(n_samples);
    let mut volume = Vec::with_capacity(n_samples);

    for i in 0..n_samples {
        let t = i as f64 * dt;
        times.push(t);
        let phase = (t / cycle_time).rem_euclid(1.0);
        let mut v = breath_volume_at_phase(
            phase,
            f_insp,
            params.kappa_insp,
            params.tau_exp,
            tidal,
        );
        // Small zero-mean noise on volume (not on flow — avoids random-walk baseline)
        if params.noise_level > 0.0 {
            v += (rng.random::<f64>() * 2.0 - 1.0) * params.noise_level * tidal;
        }
        volume.push(v.max(0.0)); // physiological volumes ≥ FRC baseline (0 here)
    }

    // Flow ≈ dV/dt (central differences interior)
    let mut flow = vec![0.0; n_samples];
    if n_samples >= 2 {
        flow[0] = (volume[1] - volume[0]) / dt;
        for i in 1..n_samples - 1 {
            flow[i] = (volume[i + 1] - volume[i - 1]) / (2.0 * dt);
        }
        flow[n_samples - 1] = (volume[n_samples - 1] - volume[n_samples - 2]) / dt;
    }

    // Peaks from volume (more meaningful for tidal breathing than flow zeros)
    let mut inhalation_peaks = Vec::new();
    let mut exhalation_peaks = Vec::new();
    if n_samples >= 3 {
        for i in 1..n_samples - 1 {
            if volume[i] >= volume[i - 1] && volume[i] > volume[i + 1] && volume[i] > 0.2 * tidal
            {
                inhalation_peaks.push(i);
            }
            // local minimum near baseline at end-exp
            if volume[i] <= volume[i - 1]
                && volume[i] < volume[i + 1]
                && volume[i] < 0.25 * tidal
            {
                exhalation_peaks.push(i);
            }
        }
    }

    // Prefer phase peaks from flow when available (keeps analysis API consistent)
    let phase_peaks = super::peaks::phase_peak_indices(&flow);
    if !phase_peaks.inhalation_peak_indices.is_empty() {
        inhalation_peaks = phase_peaks.inhalation_peak_indices;
    }
    if !phase_peaks.exhalation_peak_indices.is_empty() {
        exhalation_peaks = phase_peaks.exhalation_peak_indices;
    }

    RespTimeSeries {
        times,
        flow,
        volume,
        inhalation_peaks,
        exhalation_peaks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_breath_returns_to_baseline() {
        let tidal = 0.5;
        let f_insp = 1.0 / 3.0;
        let v0 = breath_volume_at_phase(0.0, f_insp, 3.2, 1.6, tidal);
        let v1 = breath_volume_at_phase(0.999999, f_insp, 3.2, 1.6, tidal);
        let vmax = (0..100)
            .map(|i| breath_volume_at_phase(i as f64 / 100.0, f_insp, 3.2, 1.6, tidal))
            .fold(0.0_f64, f64::max);
        assert!(v0.abs() < 1e-9, "start volume {v0}");
        assert!(v1.abs() < 0.02 * tidal, "end volume {v1} should ~ 0");
        assert!(
            (vmax - tidal).abs() < 0.05 * tidal,
            "peak {vmax} should ~ tidal {tidal}"
        );
    }

    #[test]
    fn respiration_volume_baseline_stable() {
        let params = RespSimulationParams {
            brpm: 12.0,
            dur_min: 1.0,
            fs: 50.0,
            noise_level: 0.0,
            seed: Some(1),
            ..Default::default()
        };
        let ts = generate_respiration_timeseries(&params);
        assert!(!ts.volume.is_empty());

        let n = ts.volume.len();
        let start = ts.volume[0];
        let end = ts.volume[n - 1];
        assert!(
            (end - start).abs() < 0.08,
            "endpoint drift: start={start:.4} end={end:.4}"
        );

        // Rolling means of first vs last 10% should be similar (no secular climb)
        let w = (n / 10).max(1);
        let mean_first: f64 = ts.volume[..w].iter().sum::<f64>() / w as f64;
        let mean_last: f64 = ts.volume[n - w..].iter().sum::<f64>() / w as f64;
        assert!(
            (mean_last - mean_first).abs() < 0.15 * params.tidal_volume,
            "window means drifted: first={mean_first:.4} last={mean_last:.4}"
        );

        let max_v = ts.volume.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min_v = ts.volume.iter().cloned().fold(f64::INFINITY, f64::min);
        let range = max_v - min_v;
        let target = params.tidal_volume * params.amplitude;
        assert!(
            range > 0.5 * target && range < 1.5 * target,
            "volume range {range:.3} should be ~ tidal {target:.3}"
        );
    }

    #[test]
    fn respiration_with_noise_stays_bounded() {
        let params = RespSimulationParams {
            brpm: 14.0,
            dur_min: 1.0,
            fs: 50.0,
            noise_level: 0.05,
            seed: Some(7),
            ..Default::default()
        };
        let ts = generate_respiration_timeseries(&params);
        let n = ts.volume.len();
        let w = (n / 10).max(1);
        let mean_first: f64 = ts.volume[..w].iter().sum::<f64>() / w as f64;
        let mean_last: f64 = ts.volume[n - w..].iter().sum::<f64>() / w as f64;
        assert!(
            (mean_last - mean_first).abs() < 0.25 * params.tidal_volume,
            "noise should not create large baseline drift: first={mean_first:.4} last={mean_last:.4}"
        );
    }
}
