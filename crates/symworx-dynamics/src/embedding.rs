// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

use symworx_stats::distance::euclidean;

/// Calculate embedding dimension.
pub fn edim(series: &[f64], m: usize, tau: usize) -> Vec<Vec<f64>> {
    let n = series.len();
    if m == 0 || tau == 0 || n < (m - 1) * tau + 1 {
        return Vec::new();
    }

    let max_index = n - (m - 1) * tau;
    let mut out = Vec::with_capacity(max_index);

    for i in 0..max_index {
        let mut v = Vec::with_capacity(m);
        for k in 0..m {
            v.push(series[i + k * tau]);
        }
        out.push(v);
    }

    out
}

/// False Nearest Neighbor's.
pub struct FnnResult {
    /// Embedding dimension.
    pub m: usize,
    /// FNN ratio.
    pub fnn_ratio: f64,
}

/// Calculate False Nearest Neighbor's.
pub fn fnn(data: &[f64], m: usize, tau: usize, rtol: f64, atol: f64, theiler: usize) -> FnnResult {
    let m0 = edim(data, m, tau);
    let m1 = edim(data, m + 1, tau);

    let n = m1.len();
    if n == 0 {
        return FnnResult {
            m,
            fnn_ratio: f64::NAN,
        };
    }

    let mut false_count = 0usize;

    for i in 0..n {
        let mut best_j = None;
        let mut best_dist = f64::INFINITY;

        for j in 0..n {
            if i == j {
                continue;
            }
            if (i as isize - j as isize).unsigned_abs() <= theiler as usize {
                continue;
            }

            let d = euclidean(&m0[i], &m0[j]);
            if d < best_dist {
                best_dist = d;
                best_j = Some(j);
            }
        }

        let j = best_j.unwrap();
        let dist_m = best_dist;
        let dist_m1 = euclidean(&m1[i], &m1[j]);

        let delta = (dist_m1 - dist_m).abs();
        let ratio = delta / dist_m;

        // Kennel test 1
        let mut is_false = ratio > rtol;

        // Kennel test 2 (absolute)
        let extra = (m1[i][m] - m1[j][m]).abs();
        if extra > atol {
            is_false = true;
        }

        if is_false {
            false_count += 1;
        }
    }

    FnnResult {
        m,
        fnn_ratio: false_count as f64 / n as f64,
    }
}
