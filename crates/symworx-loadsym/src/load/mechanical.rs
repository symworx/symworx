// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

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

    let load =
        estimate_external_load_from_pace(speeds, times, accel_count, decel_count, action_weight);

    MovementLoadMetrics {
        total_distance: distance,
        avg_speed,
        max_speed,
        accel_count,
        decel_count,
        estimated_load: load,
    }
}
