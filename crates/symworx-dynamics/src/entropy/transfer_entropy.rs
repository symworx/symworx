// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Discrete (histogram) transfer entropy.
//!
//! Bivariate Schreiber TE, joint multi-source TE, and conditional
//! (partial) TE. Continuous series are quantized with per-channel
//! quantile bins, then delay-embedded. Entropy is in nats (`ln`).
//!
//! For series that are already discrete (e.g., sleep stages, or HRV after
//! [`symworx_stats::discretize::RelativeKMeansDiscretizer`]), use the
//! `transfer_entropy_discrete*` entry points — they skip re-binning.
//! Do not concatenate users or disconnected nights into one TE series.
//!
//! This is a first-line discrete estimator, not a kNN / Kraskov
//! estimator and not a Granger test.

use std::collections::HashMap;

use symworx_math::series::discretize;

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
    /// Number of quantile bins per channel (>= 2).
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
        self.embed_params_valid() && self.bins >= 2
    }

    fn embed_params_valid(&self) -> bool {
        self.k >= 1 && self.l >= 1 && self.tau >= 1 && self.horizon >= 1
    }
}

/// Bivariate transfer entropy `TE(source -> target)`.
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

/// Joint multi-source transfer entropy `TE((X1,...,Xp) -> Y)`.
///
/// All source histories are concatenated into one conditioning block.
/// An empty source list returns `0.0`.
pub fn transfer_entropy_mv(sources: &[&[f64]], target: &[f64], cfg: &TeConfig) -> f64 {
    transfer_entropy_conditional(&[], sources, target, cfg)
}

/// Conditional (partial) transfer entropy `TE((X1,...,Xp) -> Y | Z1,...,Zq)`.
///
/// `condition` is the set held fixed; `sources` is the set whose extra
/// predictive information is measured. `sources` empty returns `0.0`.
pub fn transfer_entropy_conditional(condition: &[&[f64]], sources: &[&[f64]], target: &[f64], cfg: &TeConfig) -> f64 {
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

    te_from_bins(&cond_bins, &source_bins, &y_bins, cfg)
}

/// Bivariate transfer entropy on already-discrete series.
///
/// Labels are used as-is. [`TeConfig::bins`] is ignored; cardinality comes
/// from the values. Same embedding fields (`k`, `l`, `tau`, `horizon`) as
/// the continuous path.
pub fn transfer_entropy_discrete(source: &[u8], target: &[u8], cfg: &TeConfig) -> f64 {
    transfer_entropy_discrete_mv(&[source], target, cfg)
}

/// Joint multi-source transfer entropy on already-discrete series.
pub fn transfer_entropy_discrete_mv(sources: &[&[u8]], target: &[u8], cfg: &TeConfig) -> f64 {
    transfer_entropy_discrete_conditional(&[], sources, target, cfg)
}

/// Conditional (partial) transfer entropy on already-discrete series.
///
/// `cfg.bins` is ignored. Empty `sources` or invalid embed params return `0.0`.
pub fn transfer_entropy_discrete_conditional(
    condition: &[&[u8]],
    sources: &[&[u8]],
    target: &[u8],
    cfg: &TeConfig,
) -> f64 {
    if !cfg.embed_params_valid() || sources.is_empty() {
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

    let source_bins: Vec<Vec<u8>> = sources.iter().map(|s| s.to_vec()).collect();
    let cond_bins: Vec<Vec<u8>> = condition.iter().map(|s| s.to_vec()).collect();
    te_from_bins(&cond_bins, &source_bins, target, cfg)
}

fn te_from_bins(condition: &[Vec<u8>], sources: &[Vec<u8>], target: &[u8], cfg: &TeConfig) -> f64 {
    let n = target.len();
    let t_min = history_start(cfg.k, cfg.l, cfg.tau, condition.len() + sources.len());
    if n <= t_min + cfg.horizon {
        return 0.0;
    }

    let n_obs = n - t_min - cfg.horizon;
    let mut y_fut = Vec::with_capacity(n_obs);
    let mut y_past = Vec::with_capacity(n_obs);
    let mut z_past = Vec::with_capacity(n_obs);
    let mut xz_past = Vec::with_capacity(n_obs);

    for t in t_min..(n - cfg.horizon) {
        let fut = target[t + cfg.horizon];
        let yp = embed_at(target, t, cfg.k, cfg.tau);
        let zp = embed_sources_at(condition, t, cfg.l, cfg.tau);
        let xp = embed_sources_at(sources, t, cfg.l, cfg.tau);

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
    if te.is_finite() && te > 0.0 { te } else { 0.0 }
}

fn history_start(k: usize, l: usize, tau: usize, n_aux: usize) -> usize {
    let y_need = (k.saturating_sub(1)) * tau;
    let x_need = if n_aux == 0 { 0 } else { (l.saturating_sub(1)) * tau };
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
    let cuts = quantile_cuts(x, n_bins);
    if cuts.is_empty() {
        return Vec::new();
    }
    discretize(x, &cuts)
}

fn quantile_cuts(x: &[f64], n_bins: usize) -> Vec<f64> {
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

    let mut cuts = Vec::with_capacity(n_bins - 1);
    for i in 1..n_bins {
        let pos = (i as f64) / (n_bins as f64) * ((finite.len() - 1) as f64);
        let lo = pos.floor() as usize;
        let hi = (lo + 1).min(finite.len() - 1);
        let w = pos - lo as f64;
        let cut = finite[lo] * (1.0 - w) + finite[hi] * w;
        cuts.push(cut);
    }
    cuts
}

fn cond_entropy(a: &[Vec<u8>], b: &[Vec<u8>]) -> f64 {
    let hab = joint_entropy_pair(a, b);
    let hb = entropy_of(b);
    let h = hab - hb;
    if h.is_finite() && h > 0.0 { h } else { 0.0 }
}

fn joint_entropy_pair(a: &[Vec<u8>], b: &[Vec<u8>]) -> f64 {
    let mut keys = Vec::with_capacity(a.len());
    for (left, right) in a.iter().zip(b.iter()) {
        let mut k = left.clone();
        k.extend_from_slice(right);
        keys.push(k);
    }
    entropy_of(&keys)
}

fn entropy_of(states: &[Vec<u8>]) -> f64 {
    if states.is_empty() {
        return 0.0;
    }
    let mut counts: HashMap<&[u8], usize> = HashMap::new();
    for s in states {
        *counts.entry(s.as_slice()).or_insert(0) += 1;
    }
    let n = states.len() as f64;
    let mut h = 0.0;
    for &c in counts.values() {
        let p = c as f64 / n;
        if p > 0.0 {
            h -= p * p.ln();
        }
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn almost_eq(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    fn drive_series(n: usize, seed: u64) -> (Vec<f64>, Vec<f64>) {
        let mut s = seed;
        let mut next = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((s >> 33) as f64) / (u32::MAX as f64) - 0.5
        };
        let mut x = Vec::with_capacity(n);
        let mut y = Vec::with_capacity(n);
        let mut x_prev = 0.0;
        for _ in 0..n {
            let xi = next();
            let yi = 0.85 * x_prev + 0.15 * next();
            x.push(xi);
            y.push(yi);
            x_prev = xi;
        }
        (x, y)
    }

    #[test]
    fn short_or_mismatched_returns_zero() {
        let x = vec![0.1, 0.2];
        let y = vec![0.3, 0.4];
        assert_eq!(transfer_entropy(&x, &y), 0.0);
        let long = vec![0.0; 50];
        assert_eq!(transfer_entropy(&x, &long), 0.0);
    }

    #[test]
    fn constant_returns_zero() {
        let x = vec![1.0; 80];
        let y = vec![2.0; 80];
        assert_eq!(transfer_entropy(&x, &y), 0.0);
    }

    #[test]
    fn invalid_config_returns_zero() {
        let x = vec![0.0; 80];
        let y: Vec<f64> = (0..80).map(|i| i as f64).collect();
        let mut cfg = TeConfig::default();
        cfg.bins = 1;
        assert_eq!(transfer_entropy_with(&x, &y, &cfg), 0.0);
    }

    #[test]
    fn coupled_source_predicts_target() {
        let (x, y) = drive_series(400, 7);
        let cfg = TeConfig {
            k: 1,
            l: 1,
            tau: 1,
            horizon: 1,
            bins: 4,
        };
        let te_xy = transfer_entropy_with(&x, &y, &cfg);
        let te_yx = transfer_entropy_with(&y, &x, &cfg);
        assert!(te_xy > 0.0, "expected TE(x->y) > 0, got {te_xy}");
        assert!(te_xy > te_yx, "expected TE(x->y)={te_xy} > TE(y->x)={te_yx}");
    }

    #[test]
    fn independent_series_near_zero() {
        let n = 300usize;
        let mut s = 99u64;
        let mut next = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((s >> 33) as f64) / (u32::MAX as f64)
        };
        let x: Vec<f64> = (0..n).map(|_| next()).collect();
        let y: Vec<f64> = (0..n).map(|_| next()).collect();
        let te = transfer_entropy_with(&x, &y, &TeConfig::default());
        assert!(te < 0.15, "independent TE should be small, got {te}");
    }

    #[test]
    fn multivariate_joint_detects_the_driver() {
        let (x, y) = drive_series(400, 11);
        let n = y.len();
        let mut s = 123u64;
        let mut next = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((s >> 33) as f64) / (u32::MAX as f64) - 0.5
        };
        let z: Vec<f64> = (0..n).map(|_| next()).collect();
        let cfg = TeConfig::default();

        let te_x = transfer_entropy_with(&x, &y, &cfg);
        let te_z = transfer_entropy_with(&z, &y, &cfg);
        let te_joint = transfer_entropy_mv(&[&x, &z], &y, &cfg);
        let te_x_given_z = transfer_entropy_conditional(&[&z], &[&x], &y, &cfg);
        let te_z_given_x = transfer_entropy_conditional(&[&x], &[&z], &y, &cfg);

        assert!(te_x > te_z, "driver x should beat noise z: {te_x} vs {te_z}");
        assert!(te_joint > 0.0);
        assert!(
            te_x_given_z > te_z_given_x,
            "partial TE should keep the driver: {te_x_given_z} vs {te_z_given_x}"
        );
    }

    #[test]
    fn mv_empty_sources_zero() {
        let y = vec![0.1; 50];
        assert_eq!(transfer_entropy_mv(&[], &y, &TeConfig::default()), 0.0);
    }

    #[test]
    fn discrete_coupled_source_predicts_target() {
        let n = 400usize;
        let mut source = Vec::with_capacity(n);
        let mut target = Vec::with_capacity(n);
        let mut prev = 0u8;
        let mut s = 7u64;
        let mut next = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            s
        };
        for _ in 0..n {
            let xi = (next() % 4) as u8;
            let yi = if next() % 10 < 8 { prev } else { (next() % 4) as u8 };
            source.push(xi);
            target.push(yi);
            prev = xi;
        }
        let cfg = TeConfig::default();
        let te_xy = transfer_entropy_discrete(&source, &target, &cfg);
        let te_yx = transfer_entropy_discrete(&target, &source, &cfg);
        assert!(te_xy > 0.0, "expected discrete TE(x->y) > 0, got {te_xy}");
        assert!(te_xy > te_yx, "expected discrete TE(x->y)={te_xy} > TE(y->x)={te_yx}");
    }

    #[test]
    fn discrete_sleep_like_labels_are_not_rebinned() {
        // Five-class cycling target (sleep-like). Discrete TE must keep all
        // five symbols; a 4-bin quantile of the same values as f64 would merge.
        let n = 200usize;
        let sleep: Vec<u8> = (0..n).map(|i| (i % 5) as u8).collect();
        let mut hrv = vec![0u8; n];
        for i in 1..n {
            hrv[i] = if sleep[i - 1] >= 3 { 2 } else { sleep[i - 1] % 2 };
        }
        let cfg = TeConfig {
            bins: 4,
            ..TeConfig::default()
        };
        let te = transfer_entropy_discrete(&sleep, &hrv, &cfg);
        assert!(te > 0.0, "sleep-like labels should drive HRV bins, got {te}");
        let uniq: std::collections::HashSet<u8> = sleep.iter().copied().collect();
        assert_eq!(uniq.len(), 5);
    }

    #[test]
    fn discrete_bins_ignored_on_embed_only_config() {
        let x = vec![0u8, 1, 0, 1, 0, 1];
        let short_cfg = TeConfig {
            bins: 1, // invalid for the continuous path, ignored here
            ..TeConfig::default()
        };
        // series too short for default embed anyway
        assert_eq!(transfer_entropy_discrete(&x, &x, &short_cfg), 0.0);
        let bad_k = TeConfig {
            k: 0,
            ..TeConfig::default()
        };
        let long = vec![0u8; 80];
        assert_eq!(transfer_entropy_discrete(&long, &long, &bad_k), 0.0);
    }

    #[test]
    fn entropy_nonnegative() {
        let states = vec![vec![0u8], vec![0], vec![1], vec![1]];
        assert!(entropy_of(&states) > 0.0);
        assert!(almost_eq(entropy_of(&[vec![3u8], vec![3], vec![3]]), 0.0, 1e-12));
    }
}
