// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

use rand::{Rng, SeedableRng, rngs::StdRng};

/// Represents a respiration time-series signal.
#[derive(Debug, Clone)]
pub struct RespTimeSeries {
    /// Times associated with PPG time series
    pub times: Vec<f64>,
    /// Respiration (amplitude)
    pub flow: Vec<f64>,
    /// Respiratory volume
    pub volume: Vec<f64>,
    /// Peak inhalation
    pub inhalation_peaks: Vec<usize>,
    /// Peak exhalation
    pub exhalation_peaks: Vec<usize>,
}

/// Respiration simulation parameters (gamma-based inspiration + exponential expiration)
#[derive(Debug, Clone)]
pub struct RespSimulationParams {
    /// Breaths per minutes
    pub brpm: f64,
    /// Duration (min)
    pub dur_min: f64,
    /// Sampling Frequency
    pub fs: f64,
    /// Tital volume
    pub tidal_volume: f64,
    /// Inspiration/exhalation ration
    pub insp_exp_ratio: f64,
    /// Inspiration simulation parameter
    pub kappa_insp: f64,
    /// Exhalation simulation parmeter
    pub tau_exp: f64,
    /// Amplitude for simulation
    pub amplitude: f64,
    /// Noise level for simulation
    pub noise_level: f64,
    /// Seed the simulation for reproducibility
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

/// Generate a respiration timeseries using gamma-shaped inspiration
/// and exponential decay for expiration.
pub fn generate_respiration_timeseries(params: &RespSimulationParams) -> RespTimeSeries {
    let total_duration = params.dur_min * 60.0;
    let n_samples = (total_duration * params.fs).round() as usize;

    let mut times = Vec::with_capacity(n_samples);
    let mut flow = vec![0.0; n_samples];
    let mut volume = vec![0.0; n_samples];

    let cycle_time = 60.0 / params.brpm;
    let insp_time = cycle_time * (params.insp_exp_ratio / (1.0 + params.insp_exp_ratio));
    let exp_time = cycle_time / (1.0 + params.insp_exp_ratio);

    // Use accurate gamma
    let gamma_k = gamma(params.kappa_insp);
    let insp_norm =
        params.tidal_volume / (insp_time * (gamma_k / params.kappa_insp.powf(params.kappa_insp)));

    let mut rng = match params.seed {
        Some(s) => StdRng::seed_from_u64(s),
        None => StdRng::from_os_rng(),
    };

    let mut breath_start = 0.0;
    while breath_start < total_duration {
        let insp_samples = (insp_time * params.fs).round() as usize;
        let exp_samples = (exp_time * params.fs).round() as usize;

        // Inspiration phase (gamma function)
        for i in 0..insp_samples {
            let t = i as f64 / params.fs;
            let rel = t / insp_time;
            let val = params.amplitude
                * insp_norm
                * rel.powf(params.kappa_insp - 1.0)
                * (-params.kappa_insp * (rel - 1.0)).exp();

            let idx = ((breath_start + t) * params.fs).round() as usize;
            if idx < n_samples {
                flow[idx] = val;
            }
        }

        // Expiration phase (exponential decay)
        let insp_offset = insp_time;
        for i in 0..exp_samples {
            let t = i as f64 / params.fs;
            let val = -(params.amplitude / params.tau_exp) * (-t / params.tau_exp).exp();

            let idx = ((breath_start + insp_offset + t) * params.fs).round() as usize;
            if idx < n_samples {
                flow[idx] = val;
            }
        }

        breath_start += cycle_time;
    }

    // Add noise
    if params.noise_level > 0.0 {
        for v in flow.iter_mut() {
            *v += rng.r#gen::<f64>() * params.noise_level;
        }
    }

    // Integrate flow → volume (trapezoidal rule)
    let dt = 1.0 / params.fs;
    let mut cum = 0.0;
    for i in 0..n_samples {
        times.push(i as f64 * dt);
        if i > 0 {
            cum += (flow[i] + flow[i - 1]) * 0.5 * dt;
        }
        volume[i] = cum;
    }

    let phase_peaks = super::peaks::phase_peak_indices(&flow);

    RespTimeSeries {
        times,
        flow,
        volume,
        inhalation_peaks: phase_peaks.inhalation_peak_indices,
        exhalation_peaks: phase_peaks.exhalation_peak_indices,
    }
}

/// Accurate gamma function using Lanczos approximation (good for κ > 0.5)
fn gamma(x: f64) -> f64 {
    // Lanczos coefficients for g=7
    const G: f64 = 7.0;
    const P: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_576e-6,
        1.505_632_735_149_311_6e-7,
    ];

    if x < 0.5 {
        std::f64::consts::PI / ((std::f64::consts::PI * x).sin() * gamma(1.0 - x))
    } else {
        let x = x - 1.0;
        let mut y = P[0];
        for i in 1..P.len() {
            y += P[i] / (x + i as f64);
        }
        let t = x + G + 0.5;
        (2.0 * std::f64::consts::PI).sqrt() * t.powf(x + 0.5) * (-t).exp() * y
    }
}
