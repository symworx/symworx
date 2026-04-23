// biosym/src/physiology/generation/ppg.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use rand::Rng;

/// Represents a time-series signal.
pub struct TimeSeries {
    pub times: Vec<f64>,
    pub values: Vec<f64>,
    pub systolic_peaks: Vec<usize>,
    pub diastolic_peaks: Vec<usize>,
}

/// Generate a single PPG beat using two Gaussians:
///
/// # Arguments
/// - t0: start time of the beat
/// - duration: length (seconds) of the beat window
/// - fs: sampling frequency (Hz)
/// - params: (amp_s, mu_s, sigma_s, amp_d, mu_d, sigma_d)
///
/// # Returns
/// - times: vector of timestamps for the beat samples
fn single_ppg_beat(
    t0: f64,
    duration: f64,
    fs: f64,
    params: (f64, f64, f64, f64, f64, f64),
) -> (Vec<f64>, Vec<f64>) {
    let dt = 1.0 / fs;
    let n = (duration * fs).round() as usize;
    let mut times = Vec::with_capacity(n);
    let mut values = Vec::with_capacity(n);
    let (amp_s, mu_s, sigma_s, amp_d, mu_d, sigma_d) = params;

    for i in 0..n {
        let t = t0 + i as f64 * dt;
        let rel = t - t0;
        let g = |amp: f64, mu: f64, sigma: f64, x: f64| {
            amp * (-0.5 * ((x - mu) / sigma).powi(2)).exp()
        };
        let val = g(amp_s, mu_s, sigma_s, rel) + g(amp_d, mu_d, sigma_d, rel);
        times.push(t);
        values.push(val);
    }

    (times, values)
}

/// Stitch multiple ppg waveforms into one contiguous timeseries.
///
/// # Arguments
/// - rr_intervals: vector of beat intervals (seconds) between consecutive beat onsets.
///   If empty, uses constant rr = mean_rr repeated `count` times.
/// - beat_duration: sampling window per beat (should be >= max expected beat length)
/// - fs: sampling rate
/// - beat_params: Gaussian params for single beat as in single_ppg_beat
/// - jitter_phase: optional small random shift applied to each beat onset (seconds)
///
/// # Returns
/// - TimeSeries containing concatenated times, values, and detected peaks.
pub fn stitch_beats(
    start_time: f64,
    rr_intervals: &[f64],
    count: usize,
    beat_duration: f64,
    fs: f64,
    beat_params: (f64, f64, f64, f64, f64, f64),
    jitter_phase: Option<f64>,
) -> TimeSeries {
    let mut times: Vec<f64> = Vec::new();
    let mut values: Vec<f64> = Vec::new();
    let mut systolic_peaks: Vec<usize> = Vec::new();
    let mut diastolic_peaks: Vec<usize> = Vec::new();

    let mut current_t = start_time;
    let mut rng = rand::thread_rng();

    for i in 0..count {
        // apply optional jitter to onset
        let jitter = if let Some(jmax) = jitter_phase {
            rng.gen_range(-jmax..=jmax)
        } else {
            0.0
        };

        let onset = current_t + jitter;
        let (btimes, bvals) = single_ppg_beat(onset, beat_duration, fs, beat_params);

        // If overlapping appended segments, we allow concatenation; simple approach: append all samples.
        let base_index = times.len();
        times.extend(btimes.iter());
        values.extend(bvals.iter());

        // detect peaks within this beat segment (local maxima)
        let seg_len = bvals.len();
        for j in 1..(seg_len - 1) {
            if bvals[j] > bvals[j - 1] && bvals[j] > bvals[j + 1] {
                // decide systolic vs diastolic by proximity to mu_s and mu_d in params
                let rel_t = (j as f64) / fs; // relative time from onset
                let dist_s = (rel_t - beat_params.1).abs();
                let dist_d = (rel_t - beat_params.4).abs();
                let global_idx = base_index + j;
                if dist_s <= dist_d {
                    systolic_peaks.push(global_idx);
                } else {
                    diastolic_peaks.push(global_idx);
                }
            }
        }

        // advance current_t by specified RR interval or use last provided
        if !rr_intervals.is_empty() {
            let rr = if i < rr_intervals.len() {
                rr_intervals[i]
            } else {
                rr_intervals[rr_intervals.len() - 1]
            };
            current_t += rr;
        } else {
            // assume constant RR equal to beat_duration if no rr provided
            current_t += beat_duration;
        }
    }

    // Simple global peak pruning: keep only one systolic peak per beat by selecting the max near mu_s
    // (Optional step; comment out if not desired)
    // For now we keep as-is.

    TimeSeries {
        times,
        values,
        systolic_peaks,
        diastolic_peaks,
    }
}

#[cfg(test)]
mod test_ppg {
    use super::*;

    #[test]
    fn generate_and_stitch() {
        // Example parameters:
        // systolic Gaussian: amp 1.0, mu 0.2s, sigma 0.03s
        // diastolic Gaussian: amp 0.35, mu 0.45s, sigma 0.06s
        let params = (1.0, 0.2, 0.03, 0.35, 0.45, 0.06);
        let fs = 250.0;
        let beat_duration = 0.9; // 900 ms window per beat
        let rr_intervals = vec![0.8; 10]; // 75 bpm
        let ts = stitch_beats(0.0, &rr_intervals, 10, beat_duration, fs, params, Some(0.01));

        assert!(!ts.times.is_empty());
        assert!(!ts.values.is_empty());
        assert!(ts.systolic_peaks.len() > 0);
    }
}
