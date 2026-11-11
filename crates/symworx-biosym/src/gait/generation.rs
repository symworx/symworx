// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Vertical reference-channel generator for gait event detection.
//!
//! One Gaussian hump per stride. That is the event [`super::detect_gait_strides`] and
//! [`super::analyze_gait_signal`] treat as a stride, so a quiet series reports a mean
//! stride time of `stride_time()` and a cadence of `120 / stride_time()`.
//!
//! Timing comes from [`GaitParams::stride_time`]: cadence when it is set, otherwise
//! step length and walking speed. `stride_variability` is the coefficient of
//! variation of that stride time. `asymmetry` in `[0, 1]` shifts the hump later
//! in the stride. Values outside that range are clamped. The gap between humps
//! stays the stride time. Mass, height, and leg length are not used.
//!
//! `vertical_amp` is the detector scale, not meter displacement. The default
//! of 1 matches the normalized amplitudes `detect_gait_strides` is tuned for.
//! `stride_peaks` are the analytic hump centers before noise. A hump wider than
//! the gap between strides can merge with its neighbors, and the detector then
//! will not return those centers.

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
    /// Analytic stride-peak indices, recorded before additive noise.
    pub stride_peaks: Vec<usize>,
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
    if !params.gait.asymmetry.is_finite() {
        return Err("asymmetry must be finite");
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

    let asymmetry = params.gait.asymmetry.clamp(0.0, 1.0);
    let phase = 0.5 + 0.2 * asymmetry;
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
        push_event(
            &mut events,
            t + stride * phase,
            params.vertical_amp,
            params.duration,
            margin,
        );
        t += stride;
    }

    let dt = 1.0 / params.fs;
    let times: Vec<f64> = (0..n).map(|i| i as f64 * dt).collect();
    let mut values = vec![0.0; n];
    let mut stride_peaks = Vec::with_capacity(events.len());
    let window = 4.0 * params.hump_sigma_s;

    for (center, amp) in events {
        let idx = (center * params.fs).round();
        if !(0.0..n as f64).contains(&idx) {
            continue;
        }
        let idx = idx as usize;
        if stride_peaks.last() == Some(&idx) {
            continue;
        }
        stride_peaks.push(idx);

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
        stride_peaks,
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
    use crate::{
        GaitSignalQuality,
        analyze_gait_signal,
        detect_gait_strides,
        detect_gait_strides_with_quality,
    };

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

    fn gaps(peaks: &[usize]) -> Vec<usize> {
        peaks.windows(2).map(|w| w[1] - w[0]).collect()
    }

    #[test]
    fn quiet_series_places_one_hump_per_stride() {
        let params = quiet();
        let ts = generate_gait_timeseries(&params).unwrap();
        assert_eq!(ts.times.len(), ts.values.len());
        assert_eq!(ts.stride_peaks.len(), 8);
        for w in ts.stride_peaks.windows(2) {
            assert_eq!(w[1] - w[0], 100);
            assert!(ts.values[w[0]] > ts.values[w[0] - 1]);
            assert!(ts.values[w[0]] > ts.values[w[0] + 1]);
        }
    }

    #[test]
    fn quiet_analysis_reports_the_generated_stride_and_cadence() {
        let params = quiet();
        let ts = generate_gait_timeseries(&params).unwrap();
        let detected = detect_gait_strides(&ts.values, params.fs);
        assert_eq!(detected.peak_indices, ts.stride_peaks);

        let analysis = analyze_gait_signal(&ts.values, params.fs);
        assert_eq!(analysis.intervals.peak_times.len(), ts.stride_peaks.len());
        assert!((analysis.stats.mean_stride_time_s - 1.0).abs() < 1e-9);
        let cadence = analysis.stats.cadence_steps_min.expect("cadence");
        assert!((cadence - 120.0).abs() < 1e-6);
    }

    #[test]
    fn default_variability_stays_above_the_detector_floor() {
        let params = GaitSimulationParams {
            seed: Some(1),
            ..GaitSimulationParams::default()
        };
        assert!((params.gait.stride_variability - 0.03).abs() < 1e-12);
        let ts = generate_gait_timeseries(&params).unwrap();
        let detected = detect_gait_strides(&ts.values, params.fs);
        assert_eq!(detected.peak_indices, ts.stride_peaks);

        let analysis = analyze_gait_signal(&ts.values, params.fs);
        assert!((analysis.stats.mean_stride_time_s - 1.0).abs() < 0.05);
        let cadence = analysis.stats.cadence_steps_min.expect("cadence");
        assert!((cadence - 120.0).abs() < 8.0);
    }

    #[test]
    fn unset_cadence_follows_speed_and_step_length() {
        let params = GaitSimulationParams {
            gait: GaitParams {
                walking_speed: 1.0,
                step_length: 0.8,
                cadence: None,
                stride_variability: 0.0,
                asymmetry: 0.0,
                ..GaitParams::default()
            },
            ..GaitSimulationParams::default()
        };
        let ts = generate_gait_timeseries(&params).unwrap();
        assert!(gaps(&ts.stride_peaks).iter().all(|&gap| gap == 160));
        let analysis = analyze_gait_signal(&ts.values, params.fs);
        assert!((analysis.stats.mean_stride_time_s - 1.6).abs() < 1e-9);
        let cadence = analysis.stats.cadence_steps_min.expect("cadence");
        assert!((cadence - 75.0).abs() < 1e-6);
    }

    #[test]
    fn explicit_cadence_overrides_speed() {
        let params = GaitSimulationParams {
            gait: GaitParams {
                walking_speed: 3.0,
                cadence: Some(60.0),
                stride_variability: 0.0,
                asymmetry: 0.0,
                ..GaitParams::default()
            },
            ..GaitSimulationParams::default()
        };
        let ts = generate_gait_timeseries(&params).unwrap();
        assert!(gaps(&ts.stride_peaks).iter().all(|&gap| gap == 200));
    }

    #[test]
    fn asymmetry_shifts_the_hump_and_keeps_one_peak_per_stride() {
        let shifted = GaitSimulationParams {
            gait: GaitParams {
                stride_variability: 0.0,
                asymmetry: 0.2,
                ..GaitParams::default()
            },
            ..GaitSimulationParams::default()
        };
        let quiet_ts = generate_gait_timeseries(&quiet()).unwrap();
        let ts = generate_gait_timeseries(&shifted).unwrap();
        assert_eq!(ts.stride_peaks.len(), quiet_ts.stride_peaks.len());
        assert!(ts.stride_peaks[0] > quiet_ts.stride_peaks[0]);
        assert!(gaps(&ts.stride_peaks).iter().all(|&gap| gap == 100));
        let detected = detect_gait_strides(&ts.values, shifted.fs);
        assert_eq!(detected.peak_indices, ts.stride_peaks);
        let analysis = analyze_gait_signal(&ts.values, shifted.fs);
        assert!((analysis.stats.mean_stride_time_s - 1.0).abs() < 1e-9);
    }

    #[test]
    fn negative_asymmetry_is_clamped_to_symmetric() {
        let params = GaitSimulationParams {
            gait: GaitParams {
                stride_variability: 0.0,
                asymmetry: -0.5,
                ..GaitParams::default()
            },
            ..GaitSimulationParams::default()
        };
        let ts = generate_gait_timeseries(&params).unwrap();
        let quiet_ts = generate_gait_timeseries(&quiet()).unwrap();
        assert_eq!(ts.stride_peaks, quiet_ts.stride_peaks);
    }

    #[test]
    fn noise_does_not_move_the_analytic_peaks() {
        let mut params = GaitSimulationParams {
            gait: GaitParams {
                stride_variability: 0.0,
                asymmetry: 0.0,
                ..GaitParams::default()
            },
            seed: Some(3),
            ..GaitSimulationParams::default()
        };
        let clean = generate_gait_timeseries(&params).unwrap();
        params.noise_std = 0.05;
        let noisy = generate_gait_timeseries(&params).unwrap();
        assert_eq!(noisy.stride_peaks, clean.stride_peaks);
        assert_ne!(noisy.values, clean.values);
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
        assert_eq!(first.stride_peaks, second.stride_peaks);
        assert_eq!(first.values, second.values);
        assert!(first.stride_peaks.len() >= 4);
    }

    #[test]
    fn short_recording_returns_an_empty_peak_list() {
        let params = GaitSimulationParams {
            duration: 0.3,
            ..quiet()
        };
        let ts = generate_gait_timeseries(&params).unwrap();
        assert_eq!(ts.values.len(), 30);
        assert!(ts.stride_peaks.is_empty());
    }

    #[test]
    fn wide_sigma_merges_neighbors_so_the_detector_drops_centers() {
        let params = GaitSimulationParams {
            duration: 6.0,
            hump_sigma_s: 0.6,
            ..quiet()
        };
        let ts = generate_gait_timeseries(&params).unwrap();
        assert!(ts.stride_peaks.len() >= 2);
        let detected = detect_gait_strides(&ts.values, params.fs);
        assert!(detected.peak_indices.len() < ts.stride_peaks.len());
    }

    #[test]
    fn reference_quality_band_inserts_extra_peaks_on_this_hump() {
        let params = quiet();
        let ts = generate_gait_timeseries(&params).unwrap();
        let detected = detect_gait_strides_with_quality(&ts.values, params.fs, GaitSignalQuality::Reference);
        // The 0.2–6 Hz preset rings on this Gaussian and inserts extra peaks.
        // The raw detector is the path that recovers `stride_peaks`.
        assert!(detected.peak_indices.len() > ts.stride_peaks.len());
        let mean = detected.intervals_sec.iter().sum::<f64>() / detected.intervals_sec.len() as f64;
        assert!(mean < 0.85, "mean interval {mean}");
    }

    #[test]
    fn rejects_unusable_inputs() {
        let cases: Vec<(GaitSimulationParams, &str)> = vec![
            (
                GaitSimulationParams {
                    fs: 0.0,
                    ..GaitSimulationParams::default()
                },
                "sample rate must be finite and positive",
            ),
            (
                GaitSimulationParams {
                    duration: 0.0,
                    ..GaitSimulationParams::default()
                },
                "duration must be finite and positive",
            ),
            (
                GaitSimulationParams {
                    vertical_amp: 0.0,
                    ..GaitSimulationParams::default()
                },
                "vertical amplitude must be finite and positive",
            ),
            (
                GaitSimulationParams {
                    hump_sigma_s: f64::NAN,
                    ..GaitSimulationParams::default()
                },
                "hump sigma must be finite and positive",
            ),
            (
                GaitSimulationParams {
                    noise_std: -0.1,
                    ..GaitSimulationParams::default()
                },
                "noise std must be finite and non-negative",
            ),
            (
                GaitSimulationParams {
                    gait: GaitParams {
                        stride_variability: -0.01,
                        ..GaitParams::default()
                    },
                    ..GaitSimulationParams::default()
                },
                "stride variability must be finite and non-negative",
            ),
            (
                GaitSimulationParams {
                    gait: GaitParams {
                        asymmetry: f64::NAN,
                        ..GaitParams::default()
                    },
                    ..GaitSimulationParams::default()
                },
                "asymmetry must be finite",
            ),
            (
                GaitSimulationParams {
                    gait: GaitParams {
                        step_length: 0.0,
                        cadence: None,
                        ..GaitParams::default()
                    },
                    ..GaitSimulationParams::default()
                },
                "stride time must be finite and positive",
            ),
        ];
        for (params, message) in cases {
            assert_eq!(generate_gait_timeseries(&params).unwrap_err(), message);
        }
    }
}
