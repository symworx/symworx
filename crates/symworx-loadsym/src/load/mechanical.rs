// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

/// Calculate mechanical load from force and velocity data
pub fn calculate_mechanical_load(force_data: &[f64], velocity_data: &[f64]) -> f64 {
    force_data
        .iter()
        .zip(velocity_data.iter())
        .map(|(f, v)| f * v)
        .sum::<f64>()
        / force_data.len() as f64
}

/// Estimate external/mechanical load from speed (pace) series and accel/decel events.
///
/// This is intended for post-hoc analysis of tracking/GPS data (from symworx-spatialsym).
///
/// - `speeds`: m/s over time (from derive_speeds)
/// - `times`: corresponding timestamps (len = speeds.len() + 1 or compatible)
/// - `accel_count`, `decel_count`: from count_accelerations_decelerations
///
/// Basic model: approximate distance covered + weighted high-intensity actions.
pub fn estimate_external_load_from_pace(
    speeds: &[f64],
    times: &[f64],
    accel_count: usize,
    decel_count: usize,
    action_weight: f64, // e.g. 5.0-10.0 meters equivalent per high intensity action
) -> f64 {
    if speeds.is_empty() || times.len() < 2 {
        return 0.0;
    }

    let mut distance = 0.0;
    for (i, &spd) in speeds.iter().enumerate() {
        let dt = if i + 1 < times.len() {
            (times[i + 1] - times[i]).max(0.0)
        } else if i < times.len() {
            (times[i] - times[i.saturating_sub(1)]).max(0.0)
        } else {
            1.0
        };
        distance += spd * dt;
    }

    let hi_events = (accel_count + decel_count) as f64 * action_weight;
    distance + hi_events
}

/// Version that incorporates per-player peak pace normalization (relative intensity).
///
/// `relative_speeds`: already normalized 0.0-1.0 (from normalize_to_peak_pace)
pub fn estimate_external_load_from_normalized_pace(
    relative_speeds: &[f64],
    times: &[f64],
    accel_count: usize,
    decel_count: usize,
    action_weight: f64,
) -> f64 {
    if relative_speeds.is_empty() || times.len() < 2 {
        return 0.0;
    }

    // Treat relative as intensity factor; integrate as "effective high intensity distance"
    let mut effective = 0.0;
    for (i, &rel) in relative_speeds.iter().enumerate() {
        let dt = if i + 1 < times.len() {
            (times[i + 1] - times[i]).max(0.0)
        } else {
            1.0
        };
        effective += rel * dt; // relative intensity * time
    }

    let hi = (accel_count + decel_count) as f64 * action_weight;
    effective + hi
}

/// Generic metrics derived from speed series + event counts.
/// This is reusable outside of spatial/GPS contexts (e.g. any 1D speed signal).
#[derive(Debug, Clone)]
pub struct MovementLoadMetrics {
    pub total_distance: f64,
    pub avg_speed: f64,
    pub max_speed: f64,
    pub accel_count: usize,
    pub decel_count: usize,
    pub estimated_load: f64,
}

/// Compute generic movement load metrics from speed data.
///
/// `speeds`: series of speeds (m/s or any unit)
/// `times`: timestamps matching speeds (len = speeds.len() + 1 typically)
/// `accel_count`, `decel_count`: pre-counted or pass 0 and compute separately
/// `action_weight`: contribution of each accel/decel event to load
pub fn compute_movement_load_metrics(
    speeds: &[f64],
    times: &[f64],
    accel_count: usize,
    decel_count: usize,
    action_weight: f64,
) -> MovementLoadMetrics {
    if speeds.is_empty() {
        return MovementLoadMetrics {
            total_distance: 0.0,
            avg_speed: 0.0,
            max_speed: 0.0,
            accel_count,
            decel_count,
            estimated_load: 0.0,
        };
    }

    let max_speed = speeds.iter().copied().fold(0.0_f64, f64::max);
    let avg_speed = speeds.iter().sum::<f64>() / speeds.len() as f64;

    let mut distance = 0.0;
    for (i, &spd) in speeds.iter().enumerate() {
        let dt = if i + 1 < times.len() {
            (times[i + 1] - times[i]).max(0.0)
        } else {
            0.0
        };
        distance += spd * dt;
    }

    let load = estimate_external_load_from_pace(speeds, times, accel_count, decel_count, action_weight);

    MovementLoadMetrics {
        total_distance: distance,
        avg_speed,
        max_speed,
        accel_count,
        decel_count,
        estimated_load: load,
    }
}

// --- Power / intensity session analysis (sport-agnostic, for LoadSym workout view) ---

/// Simple peak (max) of the series.
pub fn peak(series: &[f64]) -> f64 {
    series.iter().copied().fold(0.0_f64, f64::max)
}

/// Highest rolling maximum over a window size (e.g. best 3,5,10,30 sample "efforts").
/// For true time-based use the time-aware version with dt.
pub fn highest_rolling(series: &[f64], window: usize) -> f64 {
    if series.is_empty() || window == 0 {
        return peak(series);
    }
    if window >= series.len() {
        return series.iter().copied().sum::<f64>() / series.len() as f64; // or max?
    }
    series
        .windows(window)
        .map(|w| w.iter().copied().fold(0.0_f64, f64::max))
        .fold(0.0_f64, f64::max)
}

/// Find contiguous regions where value >= threshold.
/// Returns vec of (start_idx, end_idx) half-open.
pub fn find_exceedance_regions(series: &[f64], threshold: f64, min_duration: usize) -> Vec<(usize, usize)> {
    let mut regions = vec![];
    let mut i = 0;
    while i < series.len() {
        if series[i] >= threshold {
            let start = i;
            while i < series.len() && series[i] >= threshold {
                i += 1;
            }
            let len = i - start;
            if len >= min_duration {
                regions.push((start, i));
            }
        } else {
            i += 1;
        }
    }
    regions
}

/// Build a simple marker string for TUI viz (short bars for exceedances).
pub fn exceedance_marker_string(series: &[f64], threshold: f64) -> String {
    series.iter().map(|&v| if v >= threshold { "│" } else { "·" }).collect()
}

// ---------------------------------------------------------------------------
// Cycling power metrics for LoadSym / periodization (SRM, Garmin, Polar .fit)
// NP, IF, TSS. These turn raw power series into a daily training load value.
// ---------------------------------------------------------------------------

/// Computed ride-level power metrics. Used to derive a single "load" number
/// (TSS by default) for ACWR / calendar views and periodization.
#[derive(Debug, Clone, PartialEq)]
pub struct RideMetrics {
    /// Total elapsed time of the activity (seconds)
    pub duration_s: f64,
    /// Total mechanical work (kJ)
    pub total_work_kj: f64,
    /// Average power across samples that had power (W)
    pub avg_power: f64,
    /// Max power observed (W)
    pub max_power: f64,
    /// Normalized Power (30 s rolling, fourth-power mean)
    pub np: f64,
    /// Intensity Factor (NP / FTP)
    pub if_: f64,
    /// Training Stress Score
    pub tss: f64,
}

/// Compute NP/IF/TSS + summary stats from power + time series.
///
/// `times_s`: relative seconds (len == power.len())
/// `power`: clean or Option-mapped power values (W). Missing treated as 0.
/// `ftp_w`: rider's functional threshold power in watts (required for IF/TSS).
/// Returns zeros when no usable power or invalid ftp.
pub fn compute_ride_metrics(times_s: &[f64], power: &[f64], ftp_w: f64) -> RideMetrics {
    if ftp_w <= 0.0 || times_s.is_empty() || power.is_empty() {
        return RideMetrics {
            duration_s: 0.0,
            total_work_kj: 0.0,
            avg_power: 0.0,
            max_power: 0.0,
            np: 0.0,
            if_: 0.0,
            tss: 0.0,
        };
    }

    let n = power.len().min(times_s.len());
    if n == 0 {
        return RideMetrics {
            duration_s: 0.0,
            total_work_kj: 0.0,
            avg_power: 0.0,
            max_power: 0.0,
            np: 0.0,
            if_: 0.0,
            tss: 0.0,
        };
    }
    let power = &power[..n];
    let times_s = &times_s[..n];

    let duration_s = times_s.last().copied().unwrap_or(n as f64).max(1.0);
    let max_power = power.iter().copied().fold(0.0_f64, f64::max);
    let sum_p: f64 = power.iter().sum();
    let avg_power = sum_p / n as f64;

    // Total work (kJ) ≈ avg_power * duration / 1000
    let total_work_kj = (avg_power * duration_s) / 1000.0;

    // Normalized Power: 30-sample (assume ~1 Hz) rolling mean ^4 then ^(1/4)
    let np = if n >= 30 {
        let mut fourth_sum = 0.0;
        let mut count = 0usize;
        for w in power.windows(30) {
            let mean30 = w.iter().copied().sum::<f64>() / 30.0;
            fourth_sum += mean30.powi(4);
            count += 1;
        }
        if count > 0 {
            (fourth_sum / count as f64).powf(0.25)
        } else {
            avg_power
        }
    } else {
        if sum_p > 0.0 {
            (power.iter().map(|p| p.powi(4)).sum::<f64>() / n as f64).powf(0.25)
        } else {
            0.0
        }
    };

    let if_ = if ftp_w > 0.0 { np / ftp_w } else { 0.0 };
    // TSS = (seconds * NP * IF) / (FTP * 36)
    let tss = if ftp_w > 0.0 {
        (duration_s * np * if_) / (ftp_w * 36.0)
    } else {
        0.0
    };

    RideMetrics {
        duration_s,
        total_work_kj,
        avg_power,
        max_power,
        np: np.max(0.0),
        if_: if_.max(0.0),
        tss: tss.max(0.0),
    }
}

/// Back-compat convenience wrapper when you have an ActivityData (TUI usage).
/// Callers must depend on symworx-io separately.
pub fn compute_ride_metrics_from_activity(times_s: &[f64], power: &[Option<f64>], ftp_w: f64) -> RideMetrics {
    let p_clean: Vec<f64> = power.iter().map(|o| o.unwrap_or(0.0)).collect();
    compute_ride_metrics(times_s, &p_clean, ftp_w)
}

/// Convenience: turn RideMetrics into a scalar daily load value (TSS is the
/// recommended default for power-based rides).
pub fn ride_load_from_metrics(metrics: &RideMetrics, method: &str) -> f64 {
    match method {
        "tss" | "TSS" | "" => metrics.tss,
        "np" | "NP" => metrics.np,
        "work" | "kj" => metrics.total_work_kj,
        "duration" => metrics.duration_s / 60.0, // minutes as proxy
        _ => metrics.tss,
    }
}

#[cfg(test)]
mod power_metrics_tests {
    use super::*;

    #[test]
    fn ride_metrics_basic_constant() {
        // 300s @ 250W, FTP=300 → NP≈250, IF≈0.833, TSS ≈ (300*250*0.833)/(300*36) ≈ 5.787
        let times: Vec<f64> = (0..300).map(|i| i as f64).collect();
        let power: Vec<f64> = vec![250.0; 300];
        let m = compute_ride_metrics(&times, &power, 300.0);
        assert!((m.duration_s - 299.0).abs() < 2.0); // 0..299
        assert!(m.np > 240.0 && m.np < 260.0);
        assert!(m.if_ > 0.8 && m.if_ < 0.9);
        assert!(m.tss > 5.0 && m.tss < 7.0);
        assert!(m.avg_power > 240.0);
    }

    #[test]
    fn ride_metrics_zero_ftp_safety() {
        let times: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let power: Vec<f64> = vec![200.0; 10];
        let m = compute_ride_metrics(&times, &power, 0.0);
        assert_eq!(m.tss, 0.0);
        assert_eq!(m.if_, 0.0);
    }
}

/// Generate a simple synthetic daily load series (e.g. for demos or tests).
/// This is an *explicit* option — synthetic data is never loaded by default.
pub fn generate_demo_daily_loads(days: usize, base: f64, variation: f64) -> Vec<f64> {
    (0..days)
        .map(|i| {
            let wave = ((i as f64) * 0.5).sin() * variation;
            (base + wave).max(50.0)
        })
        .collect()
}
