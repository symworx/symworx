// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

// Generate realistic synthetic physiological data using symworx-biosym
// Run with: cargo run -p symworx-tui --example generate_biosym_demo

use std::{
    fs,
    path::Path,
};

use symworx_biosym::physiology::{
    ppg::{
        PPGNoiseConfig,
        PPGSimulationParams,
        generate_ppg_timeseries,
    },
    respiration::{
        RespSimulationParams,
        generate_respiration_timeseries,
    },
};

fn main() {
    let data_dir = Path::new("data");
    fs::create_dir_all(data_dir).expect("Failed to create data directory");

    // ============================================
    // 1. Generate a realistic PPG recording
    // ============================================
    println!("Generating resting PPG...");

    let ppg_params = PPGSimulationParams {
        fs: 250.0,
        duration: 30.0,
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

    // Create some realistic RR intervals (slightly variable)
    let mut rr_intervals = vec![0.85; 40]; // ~70 bpm
    for (i, rr) in rr_intervals.iter_mut().enumerate() {
        // Add some natural variability + respiratory sinus arrhythmia
        *rr += 0.04 * (i as f64 * 0.3).sin();
        *rr += 0.015 * rand::random::<f64>() - 0.0075;
    }

    let ppg = generate_ppg_timeseries(
        0.0,
        &rr_intervals,
        rr_intervals.len(),
        0.9,
        ppg_params.fs,
        ppg_params.beat_params,
        &ppg_params.noise_config,
    );

    // Save as simple 2-column CSV (time, ppg)
    let ppg_path = data_dir.join("demo_ppg_resting.csv");
    save_two_column_csv(&ppg_path, &ppg.times, &ppg.values, "time,ppg");
    println!("  → Saved {}", ppg_path.display());

    // ============================================
    // 2. Generate respiration data
    // ============================================
    println!("Generating respiration signal...");

    let resp_params = RespSimulationParams {
        brpm: 14.0,
        dur_min: 1.0,
        fs: 50.0,
        tidal_volume: 0.5,
        insp_exp_ratio: 1.0 / 2.0,
        kappa_insp: 2.5,
        tau_exp: 0.6,
        amplitude: 1.0,
        noise_level: 0.05,
        seed: Some(123),
    };

    let resp = generate_respiration_timeseries(&resp_params);

    let resp_path = data_dir.join("demo_respiration.csv");
    save_two_column_csv(&resp_path, &resp.times, &resp.flow, "time,flow");
    println!("  → Saved {}", resp_path.display());

    println!("\nDone! You can now load these files in symview.");
    println!("Recommended next step: Load one of them and try the Explore tab.");
}

fn save_two_column_csv(path: &Path, x: &[f64], y: &[f64], header: &str) {
    use std::io::Write;

    let mut file = std::fs::File::create(path).expect("Failed to create file");
    writeln!(file, "{}", header).unwrap();

    for (xi, yi) in x.iter().zip(y.iter()) {
        writeln!(file, "{:.6},{:.6}", xi, yi).unwrap();
    }
}
