//! Simple demonstration of GaitParams, GaitData and analysis (GaitStats + calcs).
//!
//! Run with:
//!   cargo run -p symworx-biosym --example gait_demo

use ndarray::array;
use symworx_biosym::biomechanics::gait::{GaitData, GaitParams, analyze_gait_signal};

fn main() {
    println!("=== BioSym Gait Demo ===\n");

    let mut params = GaitParams::default().with_defaults();
    println!("Default GaitParams (with leg length & cadence estimated):");
    println!("  height       : {:.2} m", params.height);
    println!("  leg_length   : {:.2} m", params.leg_length.unwrap_or(0.0));
    println!("  cadence      : {:?} steps/min", params.cadence);
    println!("  walking_speed: {:.2} m/s\n", params.walking_speed);

    // Higher-level usage with GaitData + analysis
    let mut data = GaitData::new(100.0); // 100 Hz sampling example
    data.stride_times = Some(array![0.0, 1.15, 2.28, 3.41, 4.55]);
    let intervals = data.calculate_stride_intervals().unwrap();
    let lengths = data.calculate_stride_length(Some(1.3)).unwrap();
    let cad = data.calculate_cadence().unwrap();
    let sym = data.calculate_symmetry().unwrap_or(0.0);
    let stats = data.to_gait_stats(Some(1.3));

    println!("GaitData + analysis (synthetic strides):");
    println!("  intervals (s): {:?}", intervals);
    println!("  lengths (m @1.3m/s): {:?}", lengths);
    println!("  cadence: {:.1} steps/min", cad);
    println!("  symmetry: {:.4}", sym);
    println!(
        "  GaitStats: n={} mean_t={:.3}s mean_len={:.3}m speed={:?}",
        stats.n_strides,
        stats.mean_stride_time_s,
        stats.mean_stride_length_m.unwrap_or(0.0),
        stats.gait_speed_ms
    );

    // New: signal-based detection + analysis (parity with PPG/resp)
    println!("\nSignal-based stride detection:");
    let toy_signal = vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0];
    let ga = analyze_gait_signal(&toy_signal, 10.0);
    println!(
        "  GaitAnalysis: n_strides={}, mean interval={:.2}s",
        ga.stats.n_strides, ga.stats.mean_stride_time_s
    );

    println!("\nDemo complete.");
}
