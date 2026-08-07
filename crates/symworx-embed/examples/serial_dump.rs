// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Dump enriched JSON samples from a serial PPG device.
//!
//! Requires feature `serial` and a device emitting compatible JSON lines.
//!
//! ```bash
//! cargo run -p symworx-embed --features serial --example serial_dump -- \
//!   --port /dev/ttyACM0 --sid S001
//! ```

use symworx_embed::{
    StreamSource,
    analyze_vitals,
    sample_to_json_line,
    serial::{
        SerialConfig,
        SerialSource,
    },
};

fn main() {
    let mut port = "/dev/ttyACM0".to_string();
    let mut sid = "S001".to_string();
    let mut baud = 115_200u32;

    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--port" => {
                if let Some(v) = args.next() {
                    port = v;
                }
            }
            "--sid" => {
                if let Some(v) = args.next() {
                    sid = v;
                }
            }
            "--baud" => {
                if let Some(v) = args.next() {
                    baud = v.parse().unwrap_or(115_200);
                }
            }
            "--help" | "-h" => {
                eprintln!("Usage: serial_dump [--port /dev/ttyACM0] [--sid S001] [--baud 115200]");
                return;
            }
            other => {
                eprintln!("Unknown arg: {other} (try --help)");
                std::process::exit(2);
            }
        }
    }

    let mut cfg = SerialConfig::default();
    cfg.port = port.clone();
    cfg.sid = sid.clone();
    cfg.baud = baud;

    eprintln!("[symworx-embed] serial_dump port={port} baud={baud} sid={sid}");
    let mut src = match SerialSource::open(cfg) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ERROR opening serial: {e}");
            std::process::exit(1);
        }
    };

    loop {
        match src.next_sample() {
            Ok(Some(sample)) => {
                let status = analyze_vitals(&sample);
                let line = sample_to_json_line(&sample).expect("json");
                println!("{line}  # {}", status.as_str());
            }
            Ok(None) => {
                eprintln!("EOF on serial port");
                break;
            }
            Err(e) => {
                eprintln!("ERROR: {e}");
                break;
            }
        }
    }
}
