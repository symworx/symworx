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
    MultiWaveformDemo, // generates multiple variants of each for multi-waveform viz testing
}

impl DemoPreset {
    pub fn name(self) -> &'static str {
        match self {
            DemoPreset::RestingPPG => "Resting PPG (variants)",
            DemoPreset::LightRespiration => "Light activity respiration (variants)",
            DemoPreset::SimpleStride => "Simple stride intervals (variants)",
            DemoPreset::MultiWaveformDemo => "Multi-waveform demo (PPG x3 + resp x2 + stride x2)",
        }
    }
}

/// Generate and save demo file(s) for the given preset.
/// For multi presets, generates several variant CSVs. Returns path to a representative file.
pub fn generate_and_save(preset: DemoPreset, data_dir: &Path) -> Result<std::path::PathBuf> {
    std::fs::create_dir_all(data_dir)?;

    match preset {
        DemoPreset::RestingPPG => generate_ppg_variants(data_dir, 2),
        DemoPreset::LightRespiration => generate_respiration_variants(data_dir, 2),
        DemoPreset::SimpleStride => generate_stride_variants(data_dir, 2),
        DemoPreset::MultiWaveformDemo => generate_multi_waveform_demo(data_dir),
    }
}

fn generate_ppg_variants(data_dir: &Path, count: usize) -> Result<std::path::PathBuf> {
    // Beat-to-beat shape/height variation is applied inside biosym via noise_config.
    // Seeds are left None so each Generate run is different.
    let base_params = PPGSimulationParams {
        fs: 250.0,
        duration: 30.0,
        beat_params: (1.0, 0.18, 0.025, 0.32, 0.42, 0.055),
        noise_config: PPGNoiseConfig {
            amp_drift_std: 0.06,     // systolic/diastolic height walk
            mu_drift_std: 0.01,      // peak timing walk (s)
            sigma_drift_std: 0.008,  // width walk (s)
            onset_jitter_std: 0.006,
            global_noise_std: 0.015,
            smoothing_kernel: 5, // soften joins
        },
        seed: None,
    };

    let mut last_path = PathBuf::new();
    for i in 0..count.max(1) {
        let params = base_params.clone();
        // Slightly variable RR intervals (~70 bpm + RSA) — longer series for pan demo
        let n_beats = 80 + i * 10;
        let mut rr: Vec<f64> = (0..n_beats)
            .map(|j| 0.85 + 0.04 * (j as f64 * 0.3).sin())
            .collect();
        for r in &mut rr {
            *r += 0.02 * rand::random::<f64>() - 0.01;
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

        let path = data_dir.join(format!("demo_ppg_resting_v{}.csv", i + 1));
        save_two_column(&path, &ts.times, &ts.values, "time,ppg")?;
        last_path = path;
    }
    Ok(last_path)
}

fn generate_respiration_variants(data_dir: &Path, count: usize) -> Result<std::path::PathBuf> {
    // Closed inhale→exhale cycles in biosym (stationary tidal volume; no AC drift).
    // noise_level is relative to tidal (zero-mean on volume, not integrated flow).
    let base_params = RespSimulationParams {
        brpm: 14.0,
        dur_min: 1.0,
        fs: 50.0,
        tidal_volume: 0.5,
        insp_exp_ratio: 1.0 / 2.0, // I:E ≈ 1:2
        kappa_insp: 3.2,
        tau_exp: 1.6,
        amplitude: 1.0,
        noise_level: 0.03,
        seed: None,
    };

    let mut last_path = PathBuf::new();
    for i in 0..count.max(1) {
        let mut params = base_params.clone();
        // Mild variant spread (rate / amplitude) without locking seed
        params.brpm = 13.0 + i as f64;
        params.amplitude = 1.0 + 0.05 * i as f64;
        let ts = generate_respiration_timeseries(&params);
        let path = data_dir.join(format!("demo_respiration_v{}.csv", i + 1));
        save_two_column(&path, &ts.times, &ts.volume, "time,volume")?;
        last_path = path;
    }
    Ok(last_path)
}

fn generate_stride_variants(data_dir: &Path, count: usize) -> Result<std::path::PathBuf> {
    use rand::Rng;

    let mut last_path = PathBuf::new();
    for i in 0..count.max(1) {
        let n_steps = 120 + (i * 30);
        let base_stride = 1.07 + 0.01 * (i as f64);

        let mut rng = rand::rng();
        let mut cumulative_time = 0.0;
        let mut times = Vec::with_capacity(n_steps);
        let mut intervals = Vec::with_capacity(n_steps);

        let mut slow_drift = 0.0;

        for j in 0..n_steps {
            slow_drift = 0.035 * ((j as f64) * 0.035).sin();
            let medium = 0.022 * ((j as f64) * 0.11 + 1.3).sin();
            let noise = 0.018 * (rng.random::<f64>() - 0.5) * 2.0;
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

        let path = data_dir.join(format!("demo_stride_intervals_v{}.csv", i + 1));
        save_two_column(&path, &times, &intervals, "time,stride_time")?;
        last_path = path;
    }
    Ok(last_path)
}

fn generate_multi_waveform_demo(data_dir: &Path) -> Result<PathBuf> {
    // Generate multiple variants for PPG, respiration, and steps/stride to demo multi-waveform features
    let _ppg = generate_ppg_variants(data_dir, 3)?;
    let _resp = generate_respiration_variants(data_dir, 2)?;
    let _stride = generate_stride_variants(data_dir, 2)?;
    Ok(data_dir.join("demo_ppg_resting_v1.csv"))
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

// Compatibility shims if old fns called directly elsewhere
fn generate_ppg_resting(data_dir: &Path) -> Result<PathBuf> {
    generate_ppg_variants(data_dir, 1)
}
fn generate_respiration_light(data_dir: &Path) -> Result<PathBuf> {
    generate_respiration_variants(data_dir, 1)
}
fn generate_simple_stride(data_dir: &Path) -> Result<PathBuf> {
    generate_stride_variants(data_dir, 1)
}
