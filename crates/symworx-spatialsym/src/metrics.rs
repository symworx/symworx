// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Distance, interaction, and single-agent path-shape metrics.
//!
//! Local implementations on top of our Point2 / Vec2 types for ergonomics.

use crate::geometry::{
    Point2,
    distance as geom_distance,
};

/// Minimum path length (m) treated as a usable trajectory.
const MIN_PATH_M: f64 = 1e-12;
/// Minimum net displacement (m) for a well-defined start→end chord.
const MIN_NET_M: f64 = 1e-9;

/// Compute all pairwise Euclidean distances between agents at a single frame.
/// Returns a square matrix (n x n). Diagonal is 0.
pub fn pairwise_distances(positions: &[Point2]) -> Vec<Vec<f64>> {
    let n = positions.len();
    let mut mat = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let d = geom_distance(positions[i], positions[j]);
            mat[i][j] = d;
            mat[j][i] = d;
        }
    }
    mat
}

/// Distance from each position to a focal point (e.g. focal object).
pub fn distances_to_focal(positions: &[Point2], focal: Point2) -> Vec<f64> {
    positions.iter().map(|&p| geom_distance(p, focal)).collect()
}

/// How linear a polyline is versus its start→end chord.
///
/// `efficiency` is net displacement / path length (`1` = perfectly straight).
/// Chord deviation is omitted when the net displacement is ~0 (out-and-back /
/// loops), because the chord is then a poor reference.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathLinearity {
    /// Polyline length (meters).
    pub path_length_m: f64,
    /// Straight-line distance from first to last sample (meters).
    pub net_displacement_m: f64,
    /// `net_displacement_m / path_length_m`, in `(0, 1]`.
    pub efficiency: f64,
    /// Mean absolute perpendicular distance to the start→end chord (meters).
    pub mean_dev_m: Option<f64>,
    /// RMS perpendicular distance to the start→end chord (meters).
    pub rms_dev_m: Option<f64>,
}

fn positions_finite(positions: &[Point2]) -> bool {
    positions.iter().all(|p| p.x.is_finite() && p.y.is_finite())
}

/// Sum of consecutive segment lengths (meters).
pub fn path_length(positions: &[Point2]) -> f64 {
    positions.windows(2).map(|w| w[0].distance(w[1])).sum()
}

fn perp_distance_to_chord(p: Point2, start: Point2, chord: crate::geometry::Vec2, chord_norm: f64) -> f64 {
    let d = p - start;
    // 2-D cross magnitude / |chord|
    (d.x * chord.y - d.y * chord.x).abs() / chord_norm
}

/// Session-level path linearity of a position series.
///
/// Returns [`None`] when there are fewer than two points, any coordinate is
/// non-finite, or the path length is ~0.
///
/// # Example
/// ```
/// use symworx_spatialsym::{Point2, generate_linear_trajectory, path_linearity};
/// use symworx_spatialsym::Vec2;
///
/// let pts = generate_linear_trajectory(Point2::origin(), Vec2::new(1.0, 0.0), 5.0, 0.5);
/// let lin = path_linearity(&pts).unwrap();
/// assert!((lin.efficiency - 1.0).abs() < 1e-9);
/// assert!(lin.rms_dev_m.unwrap() < 1e-9);
/// ```
pub fn path_linearity(positions: &[Point2]) -> Option<PathLinearity> {
    if positions.len() < 2 || !positions_finite(positions) {
        return None;
    }
    let path_length_m = path_length(positions);
    if path_length_m < MIN_PATH_M {
        return None;
    }
    let start = positions[0];
    let end = *positions.last()?;
    let net_displacement_m = start.distance(end);
    let efficiency = (net_displacement_m / path_length_m).clamp(0.0, 1.0);

    let (mean_dev_m, rms_dev_m) = if net_displacement_m < MIN_NET_M {
        (None, None)
    } else {
        let chord = end - start;
        let chord_norm = chord.norm();
        let mut sum = 0.0;
        let mut sum_sq = 0.0;
        for &p in positions {
            let d = perp_distance_to_chord(p, start, chord, chord_norm);
            sum += d;
            sum_sq += d * d;
        }
        let n = positions.len() as f64;
        (Some(sum / n), Some((sum_sq / n).sqrt()))
    };

    Some(PathLinearity {
        path_length_m,
        net_displacement_m,
        efficiency,
        mean_dev_m,
        rms_dev_m,
    })
}

/// Rolling path-linearity scores over successive slices of at least `window_m`.
///
/// For each start sample, the slice ends at the first later sample whose
/// cumulative arc length is ≥ `window_m`. Leftover stubs shorter than the
/// window are omitted (not padded). Returns an empty vec when `window_m` is
/// not finite and positive.
pub fn path_linearity_windows(positions: &[Point2], window_m: f64) -> Vec<PathLinearity> {
    if positions.len() < 2 || !window_m.is_finite() || window_m <= 0.0 {
        return Vec::new();
    }

    let mut out = Vec::new();
    for start in 0..positions.len().saturating_sub(1) {
        let mut acc = 0.0;
        let mut end = None;
        for j in (start + 1)..positions.len() {
            acc += positions[j].distance(positions[j - 1]);
            if acc + f64::EPSILON >= window_m {
                end = Some(j);
                break;
            }
        }
        if let Some(j) = end
            && let Some(lin) = path_linearity(&positions[start..=j])
        {
            out.push(lin);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point2;

    #[test]
    fn pairwise_and_focal() {
        let pts = vec![Point2::new(0., 0.), Point2::new(3., 0.), Point2::new(0., 4.)];
        let dmat = pairwise_distances(&pts);
        assert!((dmat[0][1] - 3.0).abs() < 1e-9);
        assert!((dmat[0][2] - 4.0).abs() < 1e-9);
        assert!((dmat[1][2] - 5.0).abs() < 1e-9);

        let focal_d = distances_to_focal(&pts, Point2::new(0., 0.));
        assert!((focal_d[0] - 0.0).abs() < 1e-9);
    }

    #[test]
    fn linear_path_is_perfectly_efficient() {
        let pts = crate::generate_linear_trajectory(Point2::origin(), crate::Vec2::new(1.0, 0.0), 5.0, 0.5);
        let lin = path_linearity(&pts).expect("linear path");
        assert!((lin.efficiency - 1.0).abs() < 1e-9);
        assert!((lin.net_displacement_m - lin.path_length_m).abs() < 1e-9);
        assert!(lin.rms_dev_m.unwrap() < 1e-9);
        assert!(lin.mean_dev_m.unwrap() < 1e-9);
    }

    #[test]
    fn curved_path_is_less_linear_than_straight_twin() {
        let start = Point2::origin();
        let vel = crate::Vec2::new(2.0, 0.0);
        let straight = crate::generate_linear_trajectory(start, vel, 4.0, 0.05);
        let curved = crate::generate_curved_trajectory(start, vel, 4.0, 0.05, 1.5, 3.0);
        let s = path_linearity(&straight).unwrap();
        let c = path_linearity(&curved).unwrap();
        assert!(c.efficiency < s.efficiency);
        assert!(c.rms_dev_m.unwrap() > s.rms_dev_m.unwrap());
        assert!(c.path_length_m > c.net_displacement_m);
    }

    #[test]
    fn out_and_back_has_zero_efficiency_and_no_chord_dev() {
        let pts = vec![Point2::new(0.0, 0.0), Point2::new(5.0, 0.0), Point2::new(0.0, 0.0)];
        let lin = path_linearity(&pts).unwrap();
        assert!(lin.net_displacement_m < 1e-12);
        assert!(lin.efficiency < 1e-12);
        assert!(lin.mean_dev_m.is_none());
        assert!(lin.rms_dev_m.is_none());
        assert!((lin.path_length_m - 10.0).abs() < 1e-9);
    }

    #[test]
    fn distance_windows_omit_short_stub() {
        // 10 m along x at 1 m spacing → several 5 m windows, no leftover pad.
        let pts: Vec<Point2> = (0..=10).map(|i| Point2::new(i as f64, 0.0)).collect();
        let wins = path_linearity_windows(&pts, 5.0);
        assert!(!wins.is_empty());
        for w in &wins {
            assert!((w.path_length_m - 5.0).abs() < 1e-9);
            assert!((w.efficiency - 1.0).abs() < 1e-9);
        }
        assert!(path_linearity_windows(&pts, 20.0).is_empty());
        assert!(path_linearity_windows(&pts, 0.0).is_empty());
    }

    #[test]
    fn path_linearity_rejects_short_or_bad() {
        assert!(path_linearity(&[]).is_none());
        assert!(path_linearity(&[Point2::origin()]).is_none());
        assert!(path_linearity(&[Point2::new(f64::NAN, 0.0), Point2::new(1.0, 0.0)]).is_none());
    }
}
