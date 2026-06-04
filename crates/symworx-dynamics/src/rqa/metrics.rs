// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! RQA metrics, result container, and high-level entry points.

use ndarray::Array2;

use crate::rqa::{
    plot::RecurrencePlot,
    utils::{
        count_recurrences,
        find_diagonal_line_lengths,
        find_vertical_line_lengths,
        line_length_entropy,
    },
};

/// Minimum diagonal line length considered for DET / Lmax / Lentr calculations.
pub const DEFAULT_LMIN: usize = 2;

/// Minimum vertical line length considered for LAM / Vmax / trapping time.
pub const DEFAULT_VMIN: usize = 2;

/// Container for common RQA measures (following Marwan et al. conventions).
///
/// All ratio measures (RR, DET, LAM) are in [0, 1].
#[derive(Debug, Clone, Default)]
pub struct RqaResult {
    /// Recurrence rate (RR): fraction of recurrent points in the plot
    /// (after Theiler masking).
    pub recurrence_rate: f64,

    /// Determinism (DET): fraction of recurrent points that lie on
    /// diagonal lines of length ≥ `lmin`.
    pub determinism: f64,

    /// Laminarity (LAM): fraction of recurrent points that lie on
    /// vertical lines of length ≥ `vmin`.
    pub laminarity: f64,

    /// Longest diagonal line length (Lmax).
    pub lmax: usize,

    /// Average diagonal line length (Lmean) for lines ≥ lmin.
    pub lmean: f64,

    /// Shannon entropy (base 2) of the diagonal line length distribution.
    pub lentr: f64,

    /// Trapping time (TT): average length of vertical lines ≥ vmin.
    pub trapping_time: f64,

    /// Longest vertical line (Vmax).
    pub vmax: usize,

    /// Total number of recurrent points (after Theiler masking).
    pub n_recurrences: usize,
}

/// High-level RQA on a scalar time series.
///
/// Embeds the series using time-delay embedding when `m` and `tau` are provided
/// (recommended values: `m=3`, `tau=1` for many physiological signals).
///
/// # Arguments
/// * `series` — Input scalar time series.
/// * `m` — Embedding dimension (use 1 for no embedding).
/// * `tau` — Time lag for embedding.
/// * `radius` — Recurrence threshold (Euclidean distance in phase space).
/// * `theiler` — Theiler window (usually 1 or the mean period / 10).
///
/// # Returns
/// [`RqaResult`] with all standard measures (RR, DET, LAM, Lmax, Lentr, TT, ...).
///
/// See also [`rqa_from_trajectory`] for pre-embedded data.
pub fn rqa(series: &[f64], m: usize, tau: usize, radius: f64, theiler: usize) -> RqaResult {
    if m == 0 || tau == 0 || series.len() < (m - 1) * tau + 1 {
        return RqaResult::default();
    }

    let rp = RecurrencePlot::from_series(series, m, tau, radius, theiler);
    quantify(&rp.matrix, theiler)
}

/// RQA from a pre-computed trajectory matrix (rows = embedded points).
///
/// Use this when you have already performed embedding (or have multivariate
/// phase-space data) and want full control.
pub fn rqa_from_trajectory(trajectory: &Array2<f64>, radius: f64, theiler: usize) -> RqaResult {
    if trajectory.nrows() == 0 {
        return RqaResult::default();
    }

    let rp = RecurrencePlot::from_trajectory(trajectory, radius, theiler);
    quantify(&rp.matrix, theiler)
}

/// Core quantification routine. Computes all RQA measures from a binary
/// recurrence matrix (Theiler masking already applied by the plot builder).
fn quantify(matrix: &Array2<bool>, _theiler: usize) -> RqaResult {
    let n_rec = count_recurrences(matrix);
    if n_rec == 0 {
        return RqaResult::default();
    }

    let total_cells = matrix.len(); // N*N
    let rr = n_rec as f64 / total_cells as f64;

    // Diagonal lines (determinism measures)
    let diag_lengths = find_diagonal_line_lengths(matrix, DEFAULT_LMIN);
    let (det, lmax, lmean, lentr) = if diag_lengths.is_empty() {
        (0.0, 0, 0.0, 0.0)
    } else {
        let total_in_diags: usize = diag_lengths.iter().sum();
        let det = total_in_diags as f64 / n_rec as f64;
        let lmax = *diag_lengths.iter().max().unwrap_or(&0);
        let lmean = total_in_diags as f64 / diag_lengths.len() as f64;
        let lentr = line_length_entropy(&diag_lengths);
        (det, lmax, lmean, lentr)
    };

    // Vertical lines (laminarity / trapping)
    let vert_lengths = find_vertical_line_lengths(matrix, DEFAULT_VMIN);
    let (lam, vmax, tt) = if vert_lengths.is_empty() {
        (0.0, 0, 0.0)
    } else {
        let total_in_verts: usize = vert_lengths.iter().sum();
        let lam = total_in_verts as f64 / n_rec as f64;
        let vmax = *vert_lengths.iter().max().unwrap_or(&0);
        let tt = total_in_verts as f64 / vert_lengths.len() as f64;
        (lam, vmax, tt)
    };

    RqaResult {
        recurrence_rate: rr,
        determinism: det,
        laminarity: lam,
        lmax,
        lmean,
        lentr,
        trapping_time: tt,
        vmax,
        n_recurrences: n_rec,
    }
}

// TESTS
#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    fn almost_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn test_quantify_all_false_matrix() {
        let mat = Array2::from_elem((5, 5), false);
        let res = quantify(&mat, 1);
        assert_eq!(res.n_recurrences, 0);
        assert_eq!(res.recurrence_rate, 0.0);
        assert_eq!(res.determinism, 0.0);
    }

    #[test]
    fn test_small_deterministic_rp() {
        // Construct a tiny RP that has clear diagonal structure
        // (we will use the plot builder with a periodic-like trajectory)
        let traj = array![
            [0.0, 0.0],
            [1.0, 0.0],
            [0.0, 1.0],
            [1.0, 0.0], // repeats point 1
        ];

        let rp = RecurrencePlot::from_trajectory(&traj, 0.1, 0);
        let res = quantify(&rp.matrix, 0);

        // With radius 0.1 we get exact matches only on the repeated point
        assert!(res.n_recurrences >= 2); // at least the two visits to (1,0)
        // DET should be high because repeated points create diagonal lines of length 1+
    }

    #[test]
    fn test_rqa_on_sine_wave_high_determinism() {
        // Simple periodic signal should produce very high DET
        let n = 120usize;
        let series: Vec<f64> = (0..n)
            .map(|i| (2.0 * std::f64::consts::PI * i as f64 / 20.0).sin())
            .collect();

        let res = rqa(&series, 3, 2, 0.3, 3);

        // Periodic signals usually give DET > 0.7–0.9 depending on radius/Theiler
        assert!(res.determinism > 0.5, "expected high determinism for sine");
        assert!(res.lmax > 5);
        assert!(res.n_recurrences > 0);
    }

    #[test]
    fn test_rqa_from_trajectory_equivalence() {
        let series: Vec<f64> = (0..80).map(|i| (i as f64 * 0.3).sin()).collect();
        let direct = rqa(&series, 2, 1, 0.4, 1);

        // Manually embed and call lower level
        let embedded = crate::edim(&series, 2, 1);
        let n = embedded.len();
        let mut traj = Array2::<f64>::zeros((n, 2));
        for (i, v) in embedded.into_iter().enumerate() {
            traj[[i, 0]] = v[0];
            traj[[i, 1]] = v[1];
        }
        let via_traj = rqa_from_trajectory(&traj, 0.4, 1);

        assert_eq!(direct.n_recurrences, via_traj.n_recurrences);
        assert!(almost_eq(direct.determinism, via_traj.determinism));
    }

    #[test]
    fn test_rqa_short_series_returns_default() {
        let tiny = vec![1.0, 2.0];
        let res = rqa(&tiny, 5, 1, 0.1, 0); // impossible to embed
        assert_eq!(res.n_recurrences, 0);
    }
}
