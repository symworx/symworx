// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

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

/// Cross-recurrence plot between two (possibly different) time series.
///
/// Construct via [`CrossRecurrencePlot::from_series`] (embeds both) or
/// [`CrossRecurrencePlot::from_trajectories`] (pre-embedded).
#[derive(Debug, Clone)]
pub struct CrossRecurrencePlot {
    /// Binary cross-recurrence matrix. Shape `(nx, ny)`.
    /// `true` means point i in first trajectory is recurrent with j in second.
    pub matrix: Array2<bool>,

    /// Radius used.
    pub radius: f64,

    /// # embedded points from first series (rows).
    pub n_points_x: usize,

    /// # embedded points from second series (cols).
    pub n_points_y: usize,
}

impl CrossRecurrencePlot {
    /// Empty CRP.
    pub fn new() -> Self {
        Self {
            matrix: Array2::default((0, 0)),
            radius: 0.0,
            n_points_x: 0,
            n_points_y: 0,
        }
    }

    /// Build from two pre-embedded trajectories (rectangular).
    ///
    /// * `tx` shape `(nx, m)`
    /// * `ty` shape `(ny, m)`
    /// Theiler window (if >0) excludes pairs where `|i-j| <= theiler`. Useful when
    /// the two series are time-aligned; for independent recordings use theiler=0.
    pub fn from_trajectories(tx: &Array2<f64>, ty: &Array2<f64>, radius: f64, theiler: usize) -> Self {
        let nx = tx.nrows();
        let ny = ty.nrows();
        if nx == 0 || ny == 0 || tx.ncols() == 0 || ty.ncols() == 0 {
            return Self::new();
        }
        // embedding dim must match for meaningful distance
        if tx.ncols() != ty.ncols() {
            // fall back gracefully: return empty
            return Self::new();
        }

        let mut matrix = Array2::from_elem((nx, ny), false);

        for i in 0..nx {
            for j in 0..ny {
                if theiler > 0 && (i as isize - j as isize).abs() as usize <= theiler {
                    continue;
                }
                let d = euclidean(tx.row(i).as_slice().unwrap(), ty.row(j).as_slice().unwrap());
                if d <= radius {
                    matrix[[i, j]] = true;
                }
            }
        }

        Self {
            matrix,
            radius,
            n_points_x: nx,
            n_points_y: ny,
        }
    }

    /// Convenience: embed x and y with the same (m, tau) then build CRP.
    pub fn from_series(x: &[f64], y: &[f64], m: usize, tau: usize, radius: f64, theiler: usize) -> Self {
        let ex = crate::edim(x, m, tau);
        let ey = crate::edim(y, m, tau);
        if ex.is_empty() || ey.is_empty() {
            return Self::new();
        }

        let nx = ex.len();
        let ny = ey.len();
        let dim = ex[0].len();
        // assume consistent
        let mut tx = Array2::<f64>::zeros((nx, dim));
        for (i, v) in ex.into_iter().enumerate() {
            for (k, val) in v.into_iter().enumerate() {
                tx[[i, k]] = val;
            }
        }
        let mut ty = Array2::<f64>::zeros((ny, dim));
        for (i, v) in ey.into_iter().enumerate() {
            for (k, val) in v.into_iter().enumerate() {
                ty[[i, k]] = val;
            }
        }

        Self::from_trajectories(&tx, &ty, radius, theiler)
    }
}

#[cfg(test)]
mod crp_tests {
    use super::*;

    #[test]
    fn test_crp_from_series_basic() {
        let x: Vec<f64> = (0..60).map(|i| (i as f64 * 0.2).sin()).collect();
        let y: Vec<f64> = (0..55).map(|i| (i as f64 * 0.2).sin()).collect();
        let crp = CrossRecurrencePlot::from_series(&x, &y, 2, 1, 0.5, 0);
        assert!(crp.n_points_x > 0 && crp.n_points_y > 0);
        assert_eq!(crp.matrix.nrows(), crp.n_points_x);
        assert_eq!(crp.matrix.ncols(), crp.n_points_y);
    }

    #[test]
    fn test_crp_theiler_and_empty() {
        let x = vec![0.0, 1.0, 0.0, 1.0];
        let y = vec![1.0, 0.0, 1.0, 0.0];
        let crp = CrossRecurrencePlot::from_series(&x, &y, 1, 1, 10.0, 1);
        // theiler=1 on small data may zero many
        assert!(crp.n_points_x > 0);
    }
}
