//! Lightweight demo data generation using symworx-biosym.
//!
//! This module provides very simple preset-based generation for the TUI.
//! The goal is convenience for demos and testing, not full modeling control.

use std::path::{Path, PathBuf};

use anyhow::Result;
use symworx_biosym::physiology::{
    ppg::{generate_ppg_timeseries, PPGNoiseConfig, PPGSimulationParams},
    respiration::{generate_respiration_timeseries, RespSimulationParams},
};

/// Available demo presets (keep this small and opinionated).
#[derive(Debug, Clone, Copy)]
pub enum DemoPreset {
    RestingPPG,
    LightRespiration,
    SimpleStride,
}

impl DemoPreset {
    pub fn name(self) -> &'static str {
        match self {
            DemoPreset::RestingPPG => "Resting PPG (30s)",
            DemoPreset::LightRespiration => "Light activity respiration",
            DemoPreset::SimpleStride => "Simple stride intervals (walking)",
        }
    }
}

/// Generate and save a demo file for the given preset.
/// Returns the path to the generated CSV.
pub fn generate_and_save(preset: DemoPreset, data_dir: &Path) -> Result<std::path::PathBuf> {
    std::fs::create_dir_all(data_dir)?;

    match preset {
        DemoPreset::RestingPPG => generate_ppg_resting(data_dir),
        DemoPreset::LightRespiration => generate_respiration_light(data_dir),
        DemoPreset::SimpleStride => generate_simple_stride(data_dir),
    }
}

fn generate_ppg_resting(data_dir: &Path) -> Result<std::path::PathBuf> {
    let params = PPGSimulationParams {
        fs: 250.0,
        duration: 60.0, // ~60s for more interesting processing demos
        beat_params: (1.0, 0.18, 0.025, 0.32, 0.42, 0.055),
        noise_config: PPGNoiseConfig {
            amp_drift_std: 0.03,
            mu_drift_std: 0.008,
            sigma_drift_std: 0.006,
            onset_jitter_std: 0.004,
            global_noise_std: 0.02,
            smoothing_kernel: 5,
        },
        seed: Some(42),
    };

    // Slightly variable RR intervals (~70 bpm + RSA) — sized for ~60s
    let mut rr: Vec<f64> = (0..80)
        .map(|i| 0.85 + 0.04 * (i as f64 * 0.3).sin())
        .collect();
    for r in &mut rr {
        *r += 0.015 * rand::random::<f64>() - 0.0075;
    }

    let ts = generate_ppg_timeseries(
        0.0,
        &rr,
        rr.len(),
        0.9,
        params.fs,
        params.beat_params,
        &params.noise_config,
    );

    let path = data_dir.join("demo_ppg_resting.csv");
    save_two_column(&path, &ts.times, &ts.values, "time,ppg")?;
    Ok(path)
}

fn generate_respiration_light(data_dir: &Path) -> Result<std::path::PathBuf> {
    let params = RespSimulationParams {
        brpm: 14.0,
        dur_min: 1.0,
        fs: 50.0,
        tidal_volume: 0.5,
        insp_exp_ratio: 1.0 / 2.0,
        kappa_insp: 3.2,
        tau_exp: 1.6, // longer time constant → smoother, more natural expiration
        amplitude: 1.0,
        noise_level: 0.05,
        seed: Some(123),
    };

    let ts = generate_respiration_timeseries(&params);

    // We save volume (not flow) because it produces a much nicer, more
    // recognizable breathing waveform for demo/visualization purposes.
    let path = data_dir.join("demo_respiration.csv");
    save_two_column(&path, &ts.times, &ts.volume, "time,volume")?;
    Ok(path)
}

fn save_two_column(path: &Path, x: &[f64], y: &[f64], header: &str) -> Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    if !header.is_empty() {
        writeln!(f, "{}", header)?;
    }
    for (xi, yi) in x.iter().zip(y.iter()) {
        writeln!(f, "{:.6},{:.6}", xi, yi)?;
    }
    Ok(())
}

/// Generate a simple but realistic series of stride intervals.
/// This kind of data is excellent for demonstrating RQA and variability analysis.
fn generate_simple_stride(data_dir: &Path) -> Result<PathBuf> {
    use rand::Rng;

    let n_steps = 180; // ~3 minutes of walking
    let base_stride = 1.07; // seconds, typical comfortable walking

    let mut rng = rand::rng();

    let mut cumulative_time = 0.0;
    let mut times = Vec::with_capacity(n_steps);
    let mut intervals = Vec::with_capacity(n_steps);

    let mut slow_drift = 0.0; // very slow speed change

    for i in 0..n_steps {
        // Slow sinusoidal drift (fatigue / slight speed change)
        slow_drift = 0.035 * ((i as f64) * 0.035).sin();

        // Medium term variability (natural fluctuation)
        let medium = 0.022 * ((i as f64) * 0.11 + 1.3).sin();

        // Short term noise (more realistic than pure white noise)
        let noise = 0.018 * (rng.random::<f64>() - 0.5) * 2.0;

        // Occasional small "micro-adjustments"
        let micro = if rng.random::<f64>() < 0.07 {
            (rng.random::<f64>() - 0.5) * 0.045
        } else {
            0.0
        };

        let stride = base_stride + slow_drift + medium + noise + micro;

        intervals.push(stride);
        times.push(cumulative_time);
        cumulative_time += stride;
    }

    let path = data_dir.join("demo_stride_intervals.csv");
    save_two_column(&path, &times, &intervals, "time,stride_time")?;
    Ok(path)
}
