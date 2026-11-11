// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Vertical reference-channel generator for gait event detection.
//!
//! Each step is one Gaussian hump. Timing comes from [`GaitParams`]: stride time
//! from cadence (or from step length and walking speed), split into a left and
//! a right gap. `stride_variability` is the coefficient of variation of stride
//! time. `asymmetry` in `[0, 1]` lengthens the left gap and shortens the right
//! gap, and lowers the right hump. Values above 0.95 are clamped so both gaps
//! stay positive.
//!
//! `vertical_amp` is the detector scale, not a meter displacement. The default
//! of 1 matches the normalized amplitudes `detect_gait_strides` is tuned for.
//! `step_peaks` are the analytic hump centers before noise.

use rand::{
    SeedableRng,
    rngs::StdRng,
};
use symworx_math::random::sample;

use super::GaitParams;

/// Generated vertical channel and the sample index of each step hump.
#[derive(Debug, Clone, PartialEq)]
pub struct GaitTimeSeries {
    /// Sample times (seconds), uniform at `fs`.
    pub times: Vec<f64>,
    /// Vertical reference channel.
    pub values: Vec<f64>,
    /// Analytic step-peak indices, recorded before additive noise.
    pub step_peaks: Vec<usize>,
}

/// Controls for [`generate_gait_timeseries`].
#[derive(Debug, Clone, PartialEq)]
pub struct GaitSimulationParams {
    /// Sample rate (Hz).
    pub fs: f64,
    /// Recording length (seconds).
    pub duration: f64,
    /// Timing and asymmetry. Cadence is filled by [`GaitParams::with_defaults`] when missing.
    pub gait: GaitParams,
    /// Hump height on the detector scale.
    pub vertical_amp: f64,
    /// Gaussian sigma of one hump (seconds).
    pub hump_sigma_s: f64,
    /// Standard deviation of additive noise, in the same units as `vertical_amp`.
    pub noise_std: f64,
    /// Seed for variability and noise. `None` uses the OS rng when either is non-zero.
    pub seed: Option<u64>,
}

impl Default for GaitSimulationParams {
    fn default() -> Self {
        Self {
            fs: 100.0,
            duration: 8.0,
            gait: GaitParams::default(),
            vertical_amp: 1.0,
            hump_sigma_s: 0.08,
            noise_std: 0.0,
            seed: None,
        }
    }
}

/// Synthesize one vertical channel.
pub fn generate_gait_timeseries(params: &GaitSimulationParams) -> Result<GaitTimeSeries, &'static str> {
    if !params.fs.is_finite() || params.fs <= 0.0 {
        return Err("sample rate must be finite and positive");
    }
    if !params.duration.is_finite() || params.duration <= 0.0 {
        return Err("duration must be finite and positive");
    }
    if !params.vertical_amp.is_finite() || params.vertical_amp <= 0.0 {
        return Err("vertical amplitude must be finite and positive");
    }
    if !params.hump_sigma_s.is_finite() || params.hump_sigma_s <= 0.0 {
        return Err("hump sigma must be finite and positive");
    }
    if !params.noise_std.is_finite() || params.noise_std < 0.0 {
        return Err("noise std must be finite and non-negative");
    }
    if !params.gait.stride_variability.is_finite() || params.gait.stride_variability < 0.0 {
        return Err("stride variability must be finite and non-negative");
    }

    let n_f = params.duration * params.fs;
    if !n_f.is_finite() || n_f < 1.0 || n_f > usize::MAX as f64 {
        return Err("sample count is not usable");
    }
    let n = n_f.round() as usize;

    let gait = params.gait.clone().with_defaults();
    let base_stride = gait.stride_time();
    if !base_stride.is_finite() || base_stride <= 0.0 {
        return Err("stride time must be finite and positive");
    }

    let asymmetry = params.gait.asymmetry.clamp(0.0, 0.95);
    let cv = params.gait.stride_variability;
    let margin = 3.0 * params.hump_sigma_s;
    let mut rng = if cv > 0.0 || params.noise_std > 0.0 {
        Some(match params.seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_os_rng(),
        })
    } else {
        None
    };

    let mut events = Vec::new();
    let mut t = 0.0;
    while t < params.duration && events.len() < n {
        let stride = match rng.as_mut() {
            Some(rng) if cv > 0.0 => {
                let z = sample::normal(rng, 0.0, cv);
                base_stride * (1.0 + z).clamp(0.5, 1.5)
            }
            _ => base_stride,
        };
        let left_gap = stride * 0.5 * (1.0 + asymmetry);
        let right_gap = stride * 0.5 * (1.0 - asymmetry);
        t += left_gap;
        push_event(&mut events, t, params.vertical_amp, params.duration, margin);
        t += right_gap;
        let right_amp = params.vertical_amp * (1.0 - asymmetry);
        push_event(&mut events, t, right_amp, params.duration, margin);
    }

    let dt = 1.0 / params.fs;
    let times: Vec<f64> = (0..n).map(|i| i as f64 * dt).collect();
    let mut values = vec![0.0; n];
    let mut step_peaks = Vec::with_capacity(events.len());
    let window = 4.0 * params.hump_sigma_s;

    for (center, amp) in events {
        let idx = (center * params.fs).round();
        if !(0.0..n as f64).contains(&idx) {
            continue;
        }
        let idx = idx as usize;
        if step_peaks.last() == Some(&idx) {
            continue;
        }
        step_peaks.push(idx);

        let i0 = ((center - window) * params.fs).floor().max(0.0) as usize;
        let i1 = (((center + window) * params.fs).ceil() as usize).min(n);
        for i in i0..i1 {
            let d = times[i] - center;
            values[i] += amp * (-0.5 * (d / params.hump_sigma_s).powi(2)).exp();
        }
    }

    if params.noise_std > 0.0
        && let Some(rng) = rng.as_mut()
    {
        for v in &mut values {
            *v += sample::normal(rng, 0.0, params.noise_std);
        }
    }

    Ok(GaitTimeSeries {
        times,
        values,
        step_peaks,
    })
}

fn push_event(events: &mut Vec<(f64, f64)>, t: f64, amp: f64, duration: f64, margin: f64) {
    if t > margin && t < duration - margin {
        events.push((t, amp));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect_gait_strides;

    fn quiet() -> GaitSimulationParams {
        GaitSimulationParams {
            gait: GaitParams {
                stride_variability: 0.0,
                asymmetry: 0.0,
                ..GaitParams::default()
            },
            ..GaitSimulationParams::default()
        }
    }

    #[test]
    fn quiet_series_places_a_hump_on_every_step() {
        let params = quiet();
        let ts = generate_gait_timeseries(&params).unwrap();
        assert_eq!(ts.times.len(), ts.values.len());
        assert!(ts.step_peaks.len() >= 10);
        for w in ts.step_peaks.windows(2) {
            assert_eq!(w[1] - w[0], 50);
            assert!(ts.values[w[0]] > ts.values[w[0] - 1]);
            assert!(ts.values[w[0]] > ts.values[w[0] + 1]);
        }
    }

    #[test]
    fn stride_detector_recovers_the_analytic_steps() {
        let params = quiet();
        let ts = generate_gait_timeseries(&params).unwrap();
        let detected = detect_gait_strides(&ts.values, params.fs);
        assert_eq!(detected.peak_indices, ts.step_peaks);
    }

    #[test]
    fn asymmetry_alternates_the_gaps() {
        let params = GaitSimulationParams {
            gait: GaitParams {
                stride_variability: 0.0,
                asymmetry: 0.2,
                ..GaitParams::default()
            },
            ..GaitSimulationParams::default()
        };
        let ts = generate_gait_timeseries(&params).unwrap();
        let gaps: Vec<usize> = ts.step_peaks.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(gaps.len() >= 4);
        // First hump is the left step. The following gap is the shorter right step.
        assert!(gaps[0] < gaps[1]);
        assert_eq!(gaps[0], gaps[2]);
        assert_eq!(gaps[1], gaps[3]);
    }

    #[test]
    fn seeded_noise_and_variability_repeat() {
        let params = GaitSimulationParams {
            gait: GaitParams {
                stride_variability: 0.02,
                ..GaitParams::default()
            },
            noise_std: 0.01,
            seed: Some(7),
            ..GaitSimulationParams::default()
        };
        let first = generate_gait_timeseries(&params).unwrap();
        let second = generate_gait_timeseries(&params).unwrap();
        assert_eq!(first.step_peaks, second.step_peaks);
        assert_eq!(first.values, second.values);
        assert!(first.step_peaks.len() >= 4);
    }

    #[test]
    fn rejects_a_non_positive_sample_rate() {
        let params = GaitSimulationParams {
            fs: 0.0,
            ..GaitSimulationParams::default()
        };
        assert_eq!(
            generate_gait_timeseries(&params).unwrap_err(),
            "sample rate must be finite and positive"
        );
    }
}
