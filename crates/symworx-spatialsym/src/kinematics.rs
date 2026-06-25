// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Kinematics derivation: velocities, speeds, headings from trajectories.
//!
//! Uses `symworx-math::series::successive_differences` and supports both
//! constant-dt and full time arrays. Designed for post-hoc analysis with
//! historical (past) and future windows.

use symworx_math::series::successive_differences;

use crate::geometry::{
    Point2,
    Vec2,
};

/// Derive velocities assuming constant dt.
/// Returns n-1 velocities.
pub fn derive_velocities(positions: &[Point2], dt: f64) -> Vec<Vec2> {
    if positions.len() < 2 || dt <= 0.0 {
        return Vec::new();
    }
    positions
        .windows(2)
        .map(|w| {
            let d = w[1] - w[0];
            Vec2 {
                x: d.x / dt,
                y: d.y / dt,
            }
        })
        .collect()
}

/// Derive velocities using per-step delta times from a time array.
/// Returns velocities for indices 1..n (first velocity is between t[0] and t[1]).
/// Length of result is positions.len().saturating_sub(1).
///
/// Uses `symworx_math::series::successive_differences` on the coordinate series.
pub fn derive_velocities_from_times(positions: &[Point2], times: &[f64]) -> Vec<Vec2> {
    if positions.len() < 2 || times.len() != positions.len() {
        return Vec::new();
    }

    let xs: Vec<f64> = positions.iter().map(|p| p.x).collect();
    let ys: Vec<f64> = positions.iter().map(|p| p.y).collect();

    let dx = successive_differences(&xs);
    let dy = successive_differences(&ys);

    let mut vels = Vec::with_capacity(dx.len());

    for i in 0..dx.len() {
        let dt = times[i + 1] - times[i];
        if dt > 0.0 {
            vels.push(Vec2 {
                x: dx[i] / dt,
                y: dy[i] / dt,
            });
        } else {
            vels.push(Vec2::zero());
        }
    }
    vels
}

/// Compute bearing (radians) of movement over a look-back window ending at each index.
/// Returns a vector of same length as input; early entries are None when not enough history.
pub fn past_bearings(positions: &[Point2], times: &[f64], window_sec: f64) -> Vec<Option<f64>> {
    windowed_bearings(positions, times, window_sec, /* look_forward= */ false)
}

/// Compute bearing (radians) of movement over a look-ahead window starting at each index.
/// Returns a vector of same length; late entries are None.
pub fn future_bearings(positions: &[Point2], times: &[f64], window_sec: f64) -> Vec<Option<f64>> {
    windowed_bearings(positions, times, window_sec, /* look_forward= */ true)
}

fn windowed_bearings(
    positions: &[Point2],
    times: &[f64],
    window_sec: f64,
    look_forward: bool,
) -> Vec<Option<f64>> {
    let n = positions.len();
    if n == 0 || times.len() != n || window_sec <= 0.0 {
        return vec![None; n];
    }

    let mut out = vec![None; n];

    for i in 0..n {
        let t_i = times[i];
        let target_t = if look_forward {
            t_i + window_sec
        } else {
            t_i - window_sec
        };

        // Find closest index in the desired direction
        let j = if look_forward {
            // first index >= target_t, or last
            times.iter().position(|&t| t >= target_t).unwrap_or(n - 1)
        } else {
            // last index <= target_t, or first
            times.iter().rposition(|&t| t <= target_t).unwrap_or(0)
        };

        if j == i {
            continue;
        }

        // For past bearing: direction traveled *to reach current* (pos[i] - pos[j_past])
        // For future bearing: direction we *will travel* (pos[j_future] - pos[i])
        let delta = if look_forward {
            positions[j] - positions[i]
        } else {
            positions[i] - positions[j]
        };

        if delta.norm() > 1e-9 {
            let bearing = delta.bearing();
            out[i] = Some(bearing);
        }
    }
    out
}

/// Convert a velocity vector (or any Vec2) to its bearing in radians.
pub fn heading_to_bearing(vel: Vec2) -> f64 {
    vel.bearing()
}

/// Convenience: convert bearing (radians) to a human-friendly description.
pub fn bearing_to_cardinal(bearing: f64) -> &'static str {
    let b = (bearing + std::f64::consts::PI * 2.0) % (std::f64::consts::PI * 2.0);
    let deg = b.to_degrees();
    match deg {
        d if (337.5..=360.0).contains(&d) || (0.0..22.5).contains(&d) => "N",
        d if (22.5..67.5).contains(&d) => "NE",
        d if (67.5..112.5).contains(&d) => "E",
        d if (112.5..157.5).contains(&d) => "SE",
        d if (157.5..202.5).contains(&d) => "S",
        d if (202.5..247.5).contains(&d) => "SW",
        d if (247.5..292.5).contains(&d) => "W",
        d if (292.5..337.5).contains(&d) => "NW",
        _ => "?",
    }
}

/// Compute scalar speed (m/s) series.
/// Returns one speed per interval; length = positions.len().saturating_sub(1)
pub fn derive_speeds(positions: &[Point2], times: &[f64]) -> Vec<f64> {
    derive_velocities_from_times(positions, times)
        .into_iter()
        .map(|v| v.norm())
        .collect()
}

/// Count high-intensity accelerations and decelerations from a speed series.
///
/// `accel_threshold`: change in speed per second (m/s²) to count as significant (e.g. 2.0-3.0).
/// Returns (num_accelerations, num_decelerations)
pub fn count_accelerations_decelerations(
    speeds: &[f64],
    times: &[f64],
    accel_threshold: f64,
) -> (usize, usize) {
    if speeds.len() < 2 || times.len() < speeds.len() + 1 {
        return (0, 0);
    }

    let mut accels = 0usize;
    let mut decels = 0usize;

    for i in 1..speeds.len() {
        let dt = times[i + 1] - times[i]; // dt for this speed interval? adjust for consecutive speeds
        // speeds[i-1] between t[i-1],t[i]; speeds[i] between t[i],t[i+1]
        // use average or the dt between mid points; for simplicity use dt of current
        if dt <= 0.0 {
            continue;
        }
        let ds = speeds[i] - speeds[i - 1];
        let a = ds / dt;
        if a > accel_threshold {
            accels += 1;
        } else if a < -accel_threshold {
            decels += 1;
        }
    }
    (accels, decels)
}

/// Post-hoc normalization of an individual's peak pace/speed.
///
/// Returns (peak_speed, Vec<relative_pace>) where relative is speed / peak (0..1+)
pub fn normalize_to_peak_pace(speeds: &[f64]) -> (f64, Vec<f64>) {
    let peak = speeds.iter().copied().fold(0.0_f64, f64::max);
    if peak <= 0.0 {
        return (0.0, vec![0.0; speeds.len()]);
    }
    let rel = speeds.iter().map(|&s| (s / peak).min(2.0)).collect(); // cap at 2x for outliers
    (peak, rel)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point2;

    #[test]
    fn velocities_from_positions() {
        let pos = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(3.0, 0.0),
        ];
        let vels = derive_velocities(&pos, 1.0);
        assert_eq!(vels.len(), 2);
        assert!((vels[0].x - 1.0).abs() < 1e-9);
        assert!((vels[1].x - 2.0).abs() < 1e-9);
    }

    #[test]
    fn past_future_bearings_basic() {
        let times = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let pos = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(3.0, 0.0),
            Point2::new(4.0, 0.0),
        ];

        let past = past_bearings(&pos, &times, 1.0);
        let future = future_bearings(&pos, &times, 1.0);

        // At index 1, past should be east (~0.0), future east
        assert!(past[1].is_some());
        assert!((past[1].unwrap()).abs() < 0.01);
        assert!(future[1].is_some());
    }

    #[test]
    fn speeds_and_events() {
        let times = vec![0.0, 1.0, 2.0, 3.0];
        let pos = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0), // speed ~1
            Point2::new(3.0, 0.0), // speed ~2
            Point2::new(4.0, 0.0), // speed ~1
        ];
        let speeds = derive_speeds(&pos, &times);
        assert_eq!(speeds.len(), 3);
        assert!((speeds[0] - 1.0).abs() < 0.01);
        assert!((speeds[1] - 2.0).abs() < 0.01);

        let (acc, dec) = count_accelerations_decelerations(&speeds, &times, 0.5);
        assert!(acc >= 1); // 1->2 accel
        // decel may or not depending exact
    }

    #[test]
    fn peak_normalization() {
        let speeds = vec![5.0, 10.0, 7.5];
        let (peak, rel) = normalize_to_peak_pace(&speeds);
        assert!((peak - 10.0).abs() < 0.01);
        assert!((rel[1] - 1.0).abs() < 0.01);
        assert!(rel[0] < 0.6);
    }
}
