// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Recurrence Plot construction.

use ndarray::Array2;
use symworx_stats::distance::euclidean;

/// A recurrence plot (binary recurrence matrix) for a single time series.
///
/// Construct via [`RecurrencePlot::from_trajectory`] (already embedded) or
/// [`RecurrencePlot::from_series`] (uses time-delay embedding internally).
#[derive(Debug, Clone)]
pub struct RecurrencePlot {
    /// Binary recurrence matrix (`true` = recurrent point).
    ///
    /// Shape is `(N, N)` where `N` is the number of embedded points.
    /// Cells inside the Theiler window are forced to `false`.
    pub matrix: Array2<bool>,

    /// Radius (threshold) used to decide recurrence (`d <= radius`).
    pub radius: f64,

    /// Number of embedded points used to build the plot (`matrix.nrows()`).
    pub n_points: usize,
}

impl RecurrencePlot {
    /// Create an empty recurrence plot (no points).
    pub fn new() -> Self {
        Self {
            matrix: Array2::default((0, 0)),
            radius: 0.0,
            n_points: 0,
        }
    }

    /// Build a recurrence plot from an already-reconstructed trajectory matrix.
    ///
    /// # Arguments
    /// * `trajectory` — Array of shape `(N, m)` (rows = time points in phase space).
    /// * `radius` — Recurrence threshold. Points with Euclidean distance ≤ radius are recurrent.
    /// * `theiler` — Theiler window size. Pairs with `|i - j| <= theiler` are excluded.
    ///
    /// # Panics
    /// Panics if `trajectory` has zero columns (embedding dimension 0).
    pub fn from_trajectory(trajectory: &Array2<f64>, radius: f64, theiler: usize) -> Self {
        let n = trajectory.nrows();
        if n == 0 || trajectory.ncols() == 0 {
            return Self::new();
        }

        let mut matrix = Array2::from_elem((n, n), false);

        for i in 0..n {
            for j in 0..n {
                if (i as isize - j as isize).abs() as usize <= theiler {
                    continue;
                }
                let row_i = trajectory.row(i);
                let row_j = trajectory.row(j);
                let dist = euclidean(row_i.as_slice().unwrap(), row_j.as_slice().unwrap());
                if dist <= radius {
                    matrix[[i, j]] = true;
                }
            }
        }

        Self {
            matrix,
            radius,
            n_points: n,
        }
    }

    /// Build a recurrence plot directly from a scalar time series using time-delay embedding.
    ///
    /// This is a convenience wrapper around [`crate::edim`] followed by
    /// [`RecurrencePlot::from_trajectory`].
    ///
    /// # Arguments
    /// * `series` — Input scalar time series.
    /// * `m` — Embedding dimension.
    /// * `tau` — Time delay (lag).
    /// * `radius`, `theiler` — See [`RecurrencePlot::from_trajectory`].
    pub fn from_series(series: &[f64], m: usize, tau: usize, radius: f64, theiler: usize) -> Self {
        let embedded = crate::edim(series, m, tau);
        if embedded.is_empty() {
            return Self::new();
        }

        let n = embedded.len();
        let dim = embedded[0].len();
        let mut traj = Array2::<f64>::zeros((n, dim));
        for (i, vec) in embedded.into_iter().enumerate() {
            for (k, val) in vec.into_iter().enumerate() {
                traj[[i, k]] = val;
            }
        }

        Self::from_trajectory(&traj, radius, theiler)
    }
}

// TESTS
#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    #[test]
    fn test_rp_from_small_trajectory_theiler() {
        // 3 points, theiler=1 should kill the main diagonal and immediate neighbors
        let traj = array![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0],];

        let rp = RecurrencePlot::from_trajectory(&traj, 10.0, 1);
        assert_eq!(rp.n_points, 3);
        assert_eq!(rp.matrix.shape(), &[3, 3]);

        // With theiler=1, only (0,2) and (2,0) can possibly be true (dist=sqrt(2)≈1.41)
        // (0,0),(1,1),(2,2) are killed by theiler
        // (0,1),(1,0),(1,2),(2,1) are killed by theiler
        // So matrix should be all false for radius=10 even.
        // With radius large, the only possible non-theiler pairs are the corners.
        // dist(0,2) = sqrt( (0-0)^2 + (0-1)^2 ) = 1.0
        assert!(rp.matrix[[0, 2]] || rp.matrix[[2, 0]]); // at least one direction if radius allows
    }

    #[test]
    fn test_rp_from_series_basic() {
        let series = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let rp = RecurrencePlot::from_series(&series, 2, 1, 1.5, 0);
        assert!(rp.n_points > 0);
        assert_eq!(rp.matrix.nrows(), rp.n_points);
    }

    #[test]
    fn test_rp_empty_inputs() {
        let empty: Vec<f64> = vec![];
        let rp = RecurrencePlot::from_series(&empty, 3, 1, 0.5, 1);
        assert_eq!(rp.n_points, 0);
    }
}
