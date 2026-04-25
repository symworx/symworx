// core/src/dynamics/embedding.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use crate::statistics::distance::euclidean;

// ==========================================================
// Embedding
// =========================================================
// ----------------------------------------------------------
// Time-delay embedding (edim)
// ----------------------------------------------------------
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

// ----------------------------------------------------------
// False Nearest Neighbors
// ----------------------------------------------------------
pub struct FnnResult {
    pub m: usize,
    pub fnn_ratio: f64,
}

pub fn fnn(
    data: &[f64],
    m: usize,
    tau: usize,
    rtol: f64,
    atol: f64,
) -> FnnResult {
    let m0 = edim(data, m, tau);
    let m1 = edim(data, m + 1, tau);

    let n = m0.len();
    if n == 0 {
        return FnnResult { m, fnn_ratio: f64::NAN };
    }

    let mut false_count = 0usize;

    for i in 0..n {
        let mut best_j = None;
        let mut best_dist = f64::INFINITY;

        for j in 0..n {
            if i == j { continue; }
            let d = euclidean(&m0[i], &m0[j]);
            if d < best_dist {
                best_dist = d;
                best_j = Some(j);
            }
        }

        let j = best_j.unwrap();
        let dist_m  = best_dist;
        let dist_m1 = euclidean(&m1[i], &m1[j]);

        let delta = (dist_m1 - dist_m).abs();
        let ratio = delta / dist_m;

        if ratio > rtol || delta > atol {
            false_count += 1;
        }
    }

    FnnResult {
        m,
        fnn_ratio: false_count as f64 / n as f64,
    }
}
