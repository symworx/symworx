//! Simple demonstration of GaitParams and basic gait metrics.
//!
//! Run with:
//!   cargo run -p symworx-biosym --example gait_demo

use ndarray::array;
use symworx_biosym::biomechanics::gait::{
    GaitData,
    GaitParams,
};

fn main() {
    println!("=== BioSym Gait Demo ===\n");

    let mut params = GaitParams::default().with_defaults();
    println!("Default GaitParams (with leg length & cadence estimated):");
    println!("  height       : {:.2} m", params.height);
    println!("  leg_length   : {:.2} m", params.leg_length.unwrap_or(0.0));
    println!("  cadence      : {:?} steps/min", params.cadence);
    println!("  walking_speed: {:.2} m/s\n", params.walking_speed);

    // Higher-level usage with GaitData (recommended public API)
    let mut data = GaitData::new(100.0); // 100 Hz sampling example
    data.stride_times = Some(array![0.0, 1.15, 2.28, 3.41, 4.55]);
    data.calculate_stride_intervals();

    println!("GaitData example (with stride times set):");
    if let Some(intervals) = &data.stride_intervals {
        println!("  Stride intervals (s): {:?}", intervals);
    }

    println!("\nDemo complete.");
}
