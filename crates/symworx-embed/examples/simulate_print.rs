// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Print synthetic vitals samples (no hardware).
//!
//! ```bash
//! cargo run -p symworx-embed --example simulate_print
//! cargo run -p symworx-embed --example simulate_print -- --sid S002 --n 20
//! ```

use std::time::Duration;

use symworx_embed::{
    StreamSource,
    analyze_vitals,
    sample_to_json_line,
    simulate::{
        SimulatorConfig,
        SimulatorSource,
    },
};

fn main() {
    let mut sid = "S001".to_string();
    let mut n: Option<u64> = Some(15);
    let mut interval_ms = 100u64;

    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--sid" => {
                if let Some(v) = args.next() {
                    sid = v;
                }
            }
            "--n" => {
                if let Some(v) = args.next() {
                    n = v.parse().ok();
                }
            }
            "--interval-ms" => {
                if let Some(v) = args.next() {
                    interval_ms = v.parse().unwrap_or(100);
                }
            }
            "--help" | "-h" => {
                eprintln!(
                    "Usage: simulate_print [--sid S001] [--n 15] [--interval-ms 100]\n\
                     Use --n 0 for infinite stream (Ctrl+C to stop)."
                );
                return;
            }
            other => {
                eprintln!("Unknown arg: {other} (try --help)");
                std::process::exit(2);
            }
        }
    }

    let max_samples = match n {
        Some(0) => None,
        other => other,
    };

    let mut src = SimulatorSource::new(SimulatorConfig {
        sid: sid.clone(),
        interval: Duration::from_millis(interval_ms),
        max_samples,
        ..Default::default()
    });

    eprintln!("[symworx-embed] simulate_print sid={sid} interval_ms={interval_ms}");
    while let Some(sample) = src.next_sample().expect("simulator") {
        let status = analyze_vitals(&sample);
        let line = sample_to_json_line(&sample).expect("json");
        println!("{line}  # {}", status.as_str());
    }
}
