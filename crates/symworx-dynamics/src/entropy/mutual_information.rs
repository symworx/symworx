// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Histogram mutual information and lag curves.
//!
//! Equal-occupancy quantile bins, entropy in nats. The same partition
//! family as discrete transfer entropy. Not a kNN estimator.
//!
//! Bin each full series once, then hold those labels across lags.
//! [`mutual_information`] refits bins on the pairs it receives, so a
//! gappy grid can be scored by passing only the complete pairs.
//! One call is one contiguous record. Do not concatenate visits.

use std::collections::BTreeMap;

use super::transfer_entropy::quantize;

/// Default quantile bins for [`ami`], [`cross_ami`], and [`mutual_information`].
pub const DEFAULT_AMI_BINS: usize = 8;

/// Histogram mutual information `I(X; Y)` in nats.
///
/// `x` and `y` are paired samples of equal length. Bins are equal-occupancy
/// quantiles of each argument. Returns `0.0` when the inputs are shorter
/// than 2, differ in length, contain a non-finite value, are constant, or
/// `bins` is outside `2..=255`.
pub fn mutual_information(x: &[f64], y: &[f64], bins: usize) -> f64 {
    if !paired_ok(x, y, bins) {
        return 0.0;
    }
    let xb = quantize(x, bins);
    let yb = quantize(y, bins);
    if xb.is_empty() || yb.is_empty() {
        return 0.0;
    }
    mutual_information_discrete(&xb, &yb)
}

/// Histogram mutual information on labels that are already discrete.
///
/// Returns `0.0` when the slices differ in length or have fewer than 2 pairs.
pub fn mutual_information_discrete(x: &[u8], y: &[u8]) -> f64 {
    if x.len() != y.len() || x.len() < 2 {
        return 0.0;
    }
    let mi = entropy_u8(x) + entropy_u8(y) - joint_entropy_u8(x, y);
    if mi.is_finite() && mi > 0.0 { mi } else { 0.0 }
}

/// Average mutual information `I(x_t; x_{t+τ})` for `τ = 0..=max_lag`.
///
/// Index `τ` matches a delay-embedding lag. The embedding readout is
/// [`first_local_minimum`]. A coarse histogram of a smooth wave can dip
/// before the quarter-period trough, so keep the curve. Bins are fit once
/// on `x`. Returns an empty
/// curve for a short, constant, non-finite, or invalid-`bins` series.
/// Lags with fewer than 2 pairs are `0.0`.
pub fn ami(x: &[f64], max_lag: usize, bins: usize) -> Vec<f64> {
    if x.len() < 2 || !bins_ok(bins) || x.iter().any(|v| !v.is_finite()) {
        return Vec::new();
    }
    let labels = quantize(x, bins);
    if labels.is_empty() {
        return Vec::new();
    }
    lagged_mi(&labels, &labels, max_lag)
}

/// Lagged mutual information `I(driver_{t−d}; target_t)` for `d = 0..=max_lag`.
///
/// Index `d` is the same lag as a Kalman control delay: the driver is
/// shifted backward. Bins are fit once on each full series. Returns an
/// empty curve when the series differ in length or are otherwise unusable
/// (see [`ami`]). Lags with fewer than 2 pairs are `0.0`.
pub fn cross_ami(driver: &[f64], target: &[f64], max_lag: usize, bins: usize) -> Vec<f64> {
    if !paired_ok(driver, target, bins) {
        return Vec::new();
    }
    let d = quantize(driver, bins);
    let t = quantize(target, bins);
    if d.is_empty() || t.is_empty() {
        return Vec::new();
    }
    lagged_mi(&d, &t, max_lag)
}

/// First interior index `τ > 0` where `v` has strictly decreased and is
/// at most the next sample.
///
/// The endpoint is not a minimum. Non-finite samples yield `None`.
pub fn first_local_minimum(v: &[f64]) -> Option<usize> {
    if v.len() < 3 {
        return None;
    }
    for tau in 1..v.len() - 1 {
        let (prev, cur, next) = (v[tau - 1], v[tau], v[tau + 1]);
        if !prev.is_finite() || !cur.is_finite() || !next.is_finite() {
            return None;
        }
        if cur < prev && cur <= next {
            return Some(tau);
        }
    }
    None
}

fn lagged_mi(driver: &[u8], target: &[u8], max_lag: usize) -> Vec<f64> {
    let n = driver.len();
    let mut out = Vec::with_capacity(max_lag + 1);
    for lag in 0..=max_lag {
        if n <= lag + 1 {
            out.push(0.0);
            continue;
        }
        out.push(mutual_information_discrete(&driver[..n - lag], &target[lag..]));
    }
    out
}

fn paired_ok(x: &[f64], y: &[f64], bins: usize) -> bool {
    x.len() == y.len() && x.len() >= 2 && bins_ok(bins) && x.iter().chain(y.iter()).all(|v| v.is_finite())
}

fn bins_ok(bins: usize) -> bool {
    (2..=255).contains(&bins)
}

fn entropy_u8(x: &[u8]) -> f64 {
    let mut counts = [0usize; 256];
    for &v in x {
        counts[v as usize] += 1;
    }
    entropy_from_counts(counts.iter().copied(), x.len() as f64)
}

fn joint_entropy_u8(x: &[u8], y: &[u8]) -> f64 {
    let mut counts: BTreeMap<(u8, u8), usize> = BTreeMap::new();
    for (&a, &b) in x.iter().zip(y.iter()) {
        *counts.entry((a, b)).or_insert(0) += 1;
    }
    entropy_from_counts(counts.values().copied(), x.len() as f64)
}

fn entropy_from_counts(counts: impl IntoIterator<Item = usize>, n: f64) -> f64 {
    let mut h = 0.0;
    for c in counts {
        if c > 0 {
            let p = c as f64 / n;
            h -= p * p.ln();
        }
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lcg(n: usize, seed: u64) -> Vec<f64> {
        let mut s = seed;
        (0..n)
            .map(|_| {
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
                ((s >> 33) as f64) / (u32::MAX as f64)
            })
            .collect()
    }

    fn shift_copy(driver: &[f64], lag: usize) -> Vec<f64> {
        let mut target = vec![0.0; driver.len()];
        for t in lag..driver.len() {
            target[t] = driver[t - lag];
        }
        target
    }

    #[test]
    fn identical_series_beats_independent_noise() {
        let x = lcg(400, 1);
        let y = lcg(400, 2);
        let same = mutual_information(&x, &x, DEFAULT_AMI_BINS);
        let indep = mutual_information(&x, &y, DEFAULT_AMI_BINS);
        assert!(same > 1.0, "self MI {same}");
        assert!(indep < 0.2, "independent MI {indep}");
        assert!(same > indep * 5.0);
    }

    #[test]
    fn bad_pairs_are_zero() {
        assert_eq!(mutual_information(&[1.0, 2.0], &[1.0], 8), 0.0);
        assert_eq!(mutual_information(&[1.0, 2.0, 3.0], &[1.0, 2.0, f64::NAN], 8), 0.0);
        assert_eq!(mutual_information(&[1.0, 1.0, 1.0], &[0.0, 1.0, 2.0], 8), 0.0);
        assert_eq!(mutual_information(&[0.0, 1.0], &[0.0, 1.0], 1), 0.0);
        assert_eq!(mutual_information_discrete(&[0, 1], &[0]), 0.0);
    }

    #[test]
    fn cross_ami_and_ccf_style_lag_peaks_on_a_shift() {
        let driver = lcg(500, 9);
        let k = 3;
        let target = shift_copy(&driver, k);
        let curve = cross_ami(&driver, &target, 8, DEFAULT_AMI_BINS);
        let peak = curve
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i);
        assert_eq!(peak, Some(k));
        assert!(curve[k] > curve[k - 1]);
        assert!(curve[k] > curve[k + 1]);
    }

    #[test]
    fn ami_lag0_matches_self_mi_and_auto_cross() {
        let x = lcg(300, 11);
        let auto = ami(&x, 5, DEFAULT_AMI_BINS);
        let crossed = cross_ami(&x, &x, 5, DEFAULT_AMI_BINS);
        let self_mi = mutual_information(&x, &x, DEFAULT_AMI_BINS);
        assert_eq!(auto.len(), 6);
        assert!((auto[0] - self_mi).abs() < 1e-12);
        assert_eq!(auto, crossed);
    }

    #[test]
    fn mismatched_or_constant_curve_is_empty() {
        assert!(ami(&[1.0, 1.0, 1.0, 1.0], 4, 8).is_empty());
        assert!(cross_ami(&[1.0, 2.0, 3.0], &[1.0, 2.0], 2, 8).is_empty());
        assert!(ami(&[0.0, 1.0, f64::NAN, 0.2], 2, 8).is_empty());
    }

    #[test]
    fn ami_recovers_a_repeated_motif_at_its_period() {
        let motif = [0.0, 1.0, 0.0, 2.0, 3.0];
        let x: Vec<f64> = (0..400).map(|i| motif[i % motif.len()]).collect();
        let curve = ami(&x, 6, DEFAULT_AMI_BINS);
        assert!(
            (curve[5] - curve[0]).abs() < 1e-9,
            "period should restore the lag-0 entropy, curve={curve:?}"
        );
        assert!(
            curve[1] < curve[0] - 0.05,
            "one step is not a copy of this motif, curve={curve:?}"
        );
    }

    #[test]
    fn first_local_minimum_reads_a_known_dip() {
        assert_eq!(first_local_minimum(&[1.0, 0.5, 0.2, 0.4, 0.3]), Some(2));
        assert_eq!(first_local_minimum(&[3.0, 1.0, 2.0]), Some(1));
        assert_eq!(first_local_minimum(&[1.0, 0.5, 0.2]), None);
        assert_eq!(first_local_minimum(&[1.0, f64::NAN, 0.2, 0.4]), None);
    }
}
