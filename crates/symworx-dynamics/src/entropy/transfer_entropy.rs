// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Discrete (histogram) transfer entropy.
//!
//! Bivariate Schreiber TE, joint multi-source TE, and conditional
//! (partial) TE. Continuous series are quantized with per-channel
//! quantile bins, then delay-embedded. Entropy is in nats (`ln`).
//!
//! This is a first-line discrete estimator, not a kNN / Kraskov
//! estimator and not a Granger test.

use std::collections::HashMap;

/// Parameters for discrete transfer entropy.
///
/// `k` / `l` are embedding lengths (number of lagged samples) for the
/// target and each source. `tau` is the lag between those samples.
/// `horizon` is the prediction step (usually 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeConfig {
    /// Target history length (embedding dimension of `Y`).
    pub k: usize,
    /// Source history length (embedding dimension of each `X`).
    pub l: usize,
    /// Embedding delay (samples between history coordinates).
    pub tau: usize,
    /// Prediction horizon (samples ahead of the last history point).
    pub horizon: usize,
    /// Number of quantile bins per channel (≥ 2).
    pub bins: usize,
}

impl Default for TeConfig {
    fn default() -> Self {
        Self {
            k: 1,
            l: 1,
            tau: 1,
            horizon: 1,
            bins: 4,
        }
    }
}

impl TeConfig {
    fn is_valid(&self) -> bool {
        self.k >= 1 && self.l >= 1 && self.tau >= 1 && self.horizon >= 1 && self.bins >= 2
    }
}

/// Bivariate transfer entropy `TE(source → target)`.
///
/// Uses [`TeConfig::default`] (`k = l = tau = horizon = 1`, 4 bins).
/// Returns `0.0` when the series are too short, constant, or otherwise
/// uninformative.
pub fn transfer_entropy(source: &[f64], target: &[f64]) -> f64 {
    transfer_entropy_with(source, target, &TeConfig::default())
}

/// Bivariate transfer entropy with explicit configuration.
pub fn transfer_entropy_with(source: &[f64], target: &[f64], cfg: &TeConfig) -> f64 {
    transfer_entropy_mv(&[source], target, cfg)
}

/// Joint multi-source transfer entropy `TE((X₁,…,Xₚ) → Y)`.
///
/// All source histories are concatenated into one conditioning block.
/// An empty source list returns `0.0`.
pub fn transfer_entropy_mv(sources: &[&[f64]], target: &[f64], cfg: &TeConfig) -> f64 {
    transfer_entropy_conditional(&[], sources, target, cfg)
}

/// Conditional (partial) transfer entropy `TE((X₁,…,Xₚ) → Y | Z₁,…,Z_q)`.
///
/// `condition` is the set held fixed; `sources` is the set whose extra
/// predictive information is measured. `sources` empty returns `0.0`.
pub fn transfer_entropy_conditional(
    condition: &[&[f64]],
    sources: &[&[f64]],
    target: &[f64],
    cfg: &TeConfig,
) -> f64 {
    if !cfg.is_valid() || sources.is_empty() {
        return 0.0;
    }

    let n = target.len();
    if sources.iter().any(|s| s.len() != n) || condition.iter().any(|s| s.len() != n) {
        return 0.0;
    }

    let t_min = history_start(cfg.k, cfg.l, cfg.tau, condition.len() + sources.len());
    if n <= t_min + cfg.horizon {
        return 0.0;
    }

    let y_bins = quantize(target, cfg.bins);
    if y_bins.is_empty() {
        return 0.0;
    }
    let source_bins: Vec<Vec<u8>> = match quantize_all(sources, cfg.bins) {
        Some(v) => v,
        None => return 0.0,
    };
    let cond_bins: Vec<Vec<u8>> = match quantize_all(condition, cfg.bins) {
        Some(v) => v,
        None => return 0.0,
    };

    let n_obs = n - t_min - cfg.horizon;
    let mut y_fut = Vec::with_capacity(n_obs);
    let mut y_past = Vec::with_capacity(n_obs);
    let mut z_past = Vec::with_capacity(n_obs);
    let mut xz_past = Vec::with_capacity(n_obs);

    for t in t_min..(n - cfg.horizon) {
        let fut = y_bins[t + cfg.horizon];
        let yp = embed_at(&y_bins, t, cfg.k, cfg.tau);
        let zp = embed_sources_at(&cond_bins, t, cfg.l, cfg.tau);
        let xp = embed_sources_at(&source_bins, t, cfg.l, cfg.tau);

        let mut xz = zp.clone();
        xz.extend_from_slice(&xp);

        y_fut.push(vec![fut]);
        y_past.push(yp);
        z_past.push(zp);
        xz_past.push(xz);
    }

    // TE = H(Yfut | Ypast, Z) - H(Yfut | Ypast, Z, X)
    let h_yf_given_yz = cond_entropy(&y_fut, &join_states(&y_past, &z_past));
    let h_yf_given_yxz = cond_entropy(&y_fut, &join_states(&y_past, &xz_past));
    let te = h_yf_given_yz - h_yf_given_yxz;
    if te.is_finite() && te > 0.0 {
        te
    } else {
        0.0
    }
}

fn history_start(k: usize, l: usize, tau: usize, n_aux: usize) -> usize {
    let y_need = (k.saturating_sub(1)) * tau;
    let x_need = if n_aux == 0 {
        0
    } else {
        (l.saturating_sub(1)) * tau
    };
    y_need.max(x_need)
}

fn embed_at(bins: &[u8], t: usize, dim: usize, tau: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(dim);
    for i in 0..dim {
        v.push(bins[t - i * tau]);
    }
    v
}

fn embed_sources_at(channels: &[Vec<u8>], t: usize, dim: usize, tau: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(channels.len() * dim);
    for ch in channels {
        v.extend(embed_at(ch, t, dim, tau));
    }
    v
}

fn join_states(a: &[Vec<u8>], b: &[Vec<u8>]) -> Vec<Vec<u8>> {
    a.iter()
        .zip(b.iter())
        .map(|(left, right)| {
            let mut out = left.clone();
            out.extend_from_slice(right);
            out
        })
        .collect()
}

fn quantize_all(series: &[&[f64]], bins: usize) -> Option<Vec<Vec<u8>>> {
    let mut out = Vec::with_capacity(series.len());
    for s in series {
        let q = quantize(s, bins);
        if q.is_empty() {
            return None;
        }
        out.push(q);
    }
    Some(out)
}

/// Equal-occupancy (quantile) bins. Returns empty if the series is
/// degenerate (too short or fewer than two distinct finite values).
fn quantize(x: &[f64], n_bins: usize) -> Vec<u8> {
    if x.len() < 2 || n_bins < 2 {
        return Vec::new();
    }
    let mut finite: Vec<f64> = x.iter().copied().filter(|v| v.is_finite()).collect();
    if finite.len() < 2 {
        return Vec::new();
    }
    finite.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if finite[0] == *finite.last().unwrap() {
        return Vec::new();
    }

    // Interior cut points at i / n_bins, i = 1 .. n_bins-1.
    let mut cuts = Vec::with_capacity(n_bins - 1);
    for i in 1..n_bins {
        let pos = (i as f64) / (n_bins as f64) * ((finite.len() - 1) as f64);
        let lo = pos.floor() as usize;
        let hi = (lo + 1).min(finite.len() - 1);
        let w = pos - lo as f64;
        let cut = finite[lo] * (1.0 - w) + finite[hi] * w;
        cuts.push(cut);
    }

    x.iter()
        .map(|&v| {
            if !v.is_finite() {
                return 0;
            }
            let mut b = 0u8;
            for (i, &c) in cuts.iter().enumerate() {
                if v >= c {
                    b = (i as u8) + 1;
                } else {
                    break;
                }
            }
            b.min((n_bins - 1) as u8)
        })
        .collect()
}

fn cond_entropy(a: &[Vec<u8>], b: &[Vec<u8>]) -> f64 {
    // H(A|B) = H(A,B) - H(B)
    let hab = joint_entropy_pair(a, b);
    let hb = entropy_of(b);
    let h = hab - hb;
    if h.is_finite() && h > 0.0 {
        h
    } else {
        0.0
    }
}

fn joint_entropy_pair(a: &[Vec<u8>], b: &[Vec<u8>]) -> Vec<f64> {
    unreachable!()
}
