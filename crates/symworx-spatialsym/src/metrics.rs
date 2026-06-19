// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Distance and interaction metrics over trajectories.
//!
//! Local implementations of distance metrics on top of our Point2 / Vec2 types for ergonomics.

use crate::geometry::{
    Point2,
    distance as geom_distance,
};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point2;

    #[test]
    fn pairwise_and_focal() {
        let pts = vec![
            Point2::new(0., 0.),
            Point2::new(3., 0.),
            Point2::new(0., 4.),
        ];
        let dmat = pairwise_distances(&pts);
        assert!((dmat[0][1] - 3.0).abs() < 1e-9);
        assert!((dmat[0][2] - 4.0).abs() < 1e-9);
        assert!((dmat[1][2] - 5.0).abs() < 1e-9);

        let focal_d = distances_to_focal(&pts, Point2::new(0., 0.));
        assert!((focal_d[0] - 0.0).abs() < 1e-9);
    }
}
