//! Minimal demo of the new windowing + robust interpolation ("dynamics interpolation")
//! + RR-to-tachogram pipeline for 30/60 s feature work (autonomic adaptability research).
//!
//! Run with:
//!   cargo run -p symworx-signal --example windowed_rr_features

use symworx_core::math::series::{rolling_apply, time_windows};
use symworx_core::signal::processing::{
    robust_interpolate, resample_rr_to_tachogram, FillStrategy, OutlierCriterion,
};

fn main() {
    // Synthetic "raw" RR intervals (seconds) with one obvious artifact
    let mut raw_rr: Vec<f64> = (0..120)
        .map(|i| 0.85 + 0.05 * ((i as f64 * 0.1).sin()))
        .collect();
    raw_rr[47] = 2.3; // big outlier (ectopic-like)

    println!("Raw RR len = {}, has obvious spike at index 47", raw_rr.len());

    // 1. Clean using "dynamics interpolation" (Local MAD + linear interp replacement)
    let crit = OutlierCriterion::LocalMAD {
        half_window: 5,
        k: 4.0,
    };
    let cleaned = robust_interpolate(&raw_rr, crit, FillStrategy::LinearInterp);
    println!("Cleaned RR (first 5): {:?}", &cleaned[..5]);

    // 2. Turn into "event times" (cumulative) for resampling
    let mut event_times = vec![0.0f64];
    for &rr in &cleaned {
        let next = event_times.last().unwrap() + rr;
        event_times.push(next);
    }
    // Drop the last (sentinel)
    event_times.pop();

    // 3. Resample to equidistant tachogram at 4 Hz (common for HRV)
    let tach = resample_rr_to_tachogram(&event_times, &cleaned, 4.0);
    println!("Tachogram (equidistant 4 Hz) length = {}", tach.len());

    // 4. Demonstrate windowing primitives on the regular tachogram
    // 30-second windows at 4 Hz = 120 samples, step 30 s = 120 samples (non-overlap for demo)
    let win_len = 120;
    let step = 120;
    let means: Vec<f64> = rolling_apply(&tach, win_len, step, |w| {
        w.iter().sum::<f64>() / w.len() as f64
    });
    println!("Per-window means (approx RR) over ~30 s non-overlapping windows: {:?}", means);

    // Time-based segmentation example (using the original event times)
    let segments = time_windows(&event_times, 30.0, 30.0);
    println!("Time-based 30 s segments (count = {}) — first few: {:?}", segments.len(), &segments[..segments.len().min(3)]);

    println!("\nNext steps in research pipeline (not in this example):");
    println!("- Compute RMSSD / sample_entropy per window (use symworx_stats + symworx_dynamics)");
    println!("- Align windows with delta power epochs from PSG");
    println!("- Feed the multivariate window feature matrix (HR, RMSSD, SampEn, delta) into the generalized Kalman");
    println!("- Summarize the latent slow-drifting capacity trajectory per bout and relate to exercise / cognitive outcomes");
}
