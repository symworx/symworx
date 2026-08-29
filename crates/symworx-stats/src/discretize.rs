// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Grouped relative discretization.
//!
//! Fit a range per group (user), scale that group's samples onto `[0, 1]`,
//! then learn **shared** k-means cuts on the pooled unit interval. Bins mean
//! “low / mid / high **for this user**”, not raw units.
//!
//! HRV vs sleep is a typical use: pool a user’s days for the range, keep
//! sleep labels as given, then pass HRV bins to
//! `symworx_dynamics::transfer_entropy_discrete`. The API itself is
//! series-agnostic (`f64` + `group_id`).
//!
//! Do **not** concatenate users into one transfer-entropy series. Compute TE
//! per contiguous record (night), then summarize.
//!
//! Transform of a train group uses the **fitted** range (frozen). A held-out
//! group uses [`RelativeKMeansDiscretizer::transform_new_group`] (that
//! group’s own range + the shared cuts). Unknown train-group ids are an
//! error, not a silent population fallback.

use std::{
    collections::BTreeMap,
    fmt,
};

use ndarray::Array2;
use symworx_math::series::discretize;

use crate::{
    basic::percentile,
    cluster::{
        KMeansConfig,
        kmeans,
    },
};

const SPAN_EPS: f64 = 1e-12;

/// How a group’s spread is measured before scaling onto `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum RangeMethod {
    /// `min` / `max` of the group’s finite samples.
    #[default]
    MinMax,
    /// Inclusive percentile pair in `[0, 100]` (e.g. 5–95 for ectopics).
    Percentile {
        /// Lower percentile.
        lo: f64,
        /// Upper percentile.
        hi: f64,
    },
}

impl RangeMethod {
    fn validate(self) -> Result<(), DiscretizeError> {
        match self {
            RangeMethod::MinMax => Ok(()),
            RangeMethod::Percentile { lo, hi } => {
                if !(0.0..=100.0).contains(&lo) || !(0.0..=100.0).contains(&hi) || lo >= hi {
                    Err(DiscretizeError::InvalidPercentileRange { lo, hi })
                } else {
                    Ok(())
                }
            }
        }
    }
}

/// Finite min/max (or percentile edges) for one group.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroupRange {
    /// Lower edge used for scaling.
    pub min: f64,
    /// Upper edge used for scaling.
    pub max: f64,
}

impl GroupRange {
    /// `true` when the span is too small to scale.
    pub fn is_degenerate(self) -> bool {
        !(self.max - self.min).is_finite() || (self.max - self.min).abs() < SPAN_EPS
    }
}

/// Per-group range fitted on training samples.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupedRangeScaler {
    /// Range estimator used at fit time.
    pub method: RangeMethod,
    /// Fitted ranges keyed by `group_id` (sorted).
    pub ranges: BTreeMap<usize, GroupRange>,
}

impl GroupedRangeScaler {
    /// Fit one range per distinct `group_id` from finite samples.
    ///
    /// Groups with no finite values are omitted. Length mismatch is an error.
    pub fn fit(values: &[f64], group_ids: &[usize], method: RangeMethod) -> Result<Self, DiscretizeError> {
        method.validate()?;
        if values.len() != group_ids.len() {
            return Err(DiscretizeError::LengthMismatch {
                n_values: values.len(),
                n_groups: group_ids.len(),
            });
        }
        if values.is_empty() {
            return Err(DiscretizeError::EmptyInput);
        }

        let mut buckets: BTreeMap<usize, Vec<f64>> = BTreeMap::new();
        for (&v, &g) in values.iter().zip(group_ids.iter()) {
            if v.is_finite() {
                buckets.entry(g).or_default().push(v);
            }
        }

        let mut ranges = BTreeMap::new();
        for (g, samples) in buckets {
            if let Some(range) = fit_group_range(&samples, method) {
                ranges.insert(g, range);
            }
        }
        if ranges.is_empty() {
            return Err(DiscretizeError::AllGroupsDegenerate);
        }

        Ok(Self { method, ranges })
    }

    /// Range for a fitted group.
    pub fn range_of(&self, group: usize) -> Option<GroupRange> {
        self.ranges.get(&group).copied()
    }

    /// Scale `values` with each row’s fitted group range.
    ///
    /// Unknown groups error. Non-finite inputs stay non-finite.
    pub fn transform(&self, values: &[f64], group_ids: &[usize]) -> Result<Vec<f64>, DiscretizeError> {
        if values.len() != group_ids.len() {
            return Err(DiscretizeError::LengthMismatch {
                n_values: values.len(),
                n_groups: group_ids.len(),
            });
        }
        let mut out = Vec::with_capacity(values.len());
        for (&v, &g) in values.iter().zip(group_ids.iter()) {
            let range = self.range_of(g).ok_or(DiscretizeError::UnknownGroup { group: g })?;
            out.push(scale_one(v, range));
        }
        Ok(out)
    }
}

/// Shared relative k-means cuts on the pooled unit interval.
#[derive(Debug, Clone, PartialEq)]
pub struct RelativeKMeansDiscretizer {
    /// Train-group ranges (frozen at fit).
    pub scaler: GroupedRangeScaler,
    /// Number of ordinal bins (`>= 2`).
    pub n_bins: usize,
    /// Interior edges on `[0, 1]` (`n_bins - 1` of them), increasing.
    pub cuts: Vec<f64>,
    /// Sorted 1-D k-means centroids on `[0, 1]`.
    pub centroids: Vec<f64>,
}

impl RelativeKMeansDiscretizer {
    /// Fit with [`RangeMethod::MinMax`].
    ///
    /// `kmeans.k` is overwritten with `n_bins`. Other k-means fields (seed,
    /// iterations, `kmeans_pp`) are honored.
    pub fn fit(
        values: &[f64],
        group_ids: &[usize],
        n_bins: usize,
        kmeans_cfg: &KMeansConfig,
    ) -> Result<Self, DiscretizeError> {
        Self::fit_with(values, group_ids, n_bins, RangeMethod::MinMax, kmeans_cfg)
    }

    /// Fit with an explicit range method.
    pub fn fit_with(
        values: &[f64],
        group_ids: &[usize],
        n_bins: usize,
        method: RangeMethod,
        kmeans_cfg: &KMeansConfig,
    ) -> Result<Self, DiscretizeError> {
        if n_bins < 2 {
            return Err(DiscretizeError::TooFewBins { n_bins });
        }
        let scaler = GroupedRangeScaler::fit(values, group_ids, method)?;
        let scaled = scaler.transform(values, group_ids)?;
        let finite: Vec<f64> = scaled.into_iter().filter(|v| v.is_finite()).collect();
        if finite.len() < n_bins {
            return Err(DiscretizeError::TooFewFinite {
                n_finite: finite.len(),
                n_bins,
            });
        }
        let mut mn = f64::INFINITY;
        let mut mx = f64::NEG_INFINITY;
        for &v in &finite {
            mn = mn.min(v);
            mx = mx.max(v);
        }
        if mx - mn < SPAN_EPS {
            return Err(DiscretizeError::AllGroupsDegenerate);
        }

        let n = finite.len();
        let data = Array2::from_shape_vec((n, 1), finite).expect("n × 1");
        let mut cfg = kmeans_cfg.clone();
        cfg.k = n_bins;
        let km = kmeans(&data, &cfg);

        let mut centroids: Vec<f64> = (0..km.centroids.nrows()).map(|i| km.centroids[[i, 0]]).collect();
        centroids.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        if centroids.len() < 2 {
            return Err(DiscretizeError::AllGroupsDegenerate);
        }
        let cuts: Vec<f64> = centroids.windows(2).map(|w| 0.5 * (w[0] + w[1])).collect();

        Ok(Self {
            scaler,
            n_bins,
            cuts,
            centroids,
        })
    }

    /// Bin train (or other fitted-group) samples with frozen ranges + shared cuts.
    pub fn transform(&self, values: &[f64], group_ids: &[usize]) -> Result<Vec<u8>, DiscretizeError> {
        let scaled = self.scaler.transform(values, group_ids)?;
        Ok(discretize(&scaled, &self.cuts))
    }

    /// Held-out / new group: estimate that series’ own range, apply shared cuts.
    ///
    /// Empty input → empty output. A series with no finite values maps to bin `0`.
    pub fn transform_new_group(&self, values: &[f64]) -> Vec<u8> {
        if values.is_empty() {
            return Vec::new();
        }
        let range = fit_group_range(values, self.scaler.method).unwrap_or(GroupRange { min: 0.0, max: 0.0 });
        let scaled: Vec<f64> = values.iter().map(|&v| scale_one(v, range)).collect();
        discretize(&scaled, &self.cuts)
    }
}

/// Fit a range from a 1-D sample (finite values only).
///
/// Returns `None` when there are no finite points.
pub fn fit_group_range(values: &[f64], method: RangeMethod) -> Option<GroupRange> {
    let finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if finite.is_empty() {
        return None;
    }
    match method {
        RangeMethod::MinMax => {
            let mut min = f64::INFINITY;
            let mut max = f64::NEG_INFINITY;
            for &v in &finite {
                min = min.min(v);
                max = max.max(v);
            }
            Some(GroupRange { min, max })
        }
        RangeMethod::Percentile { lo, hi } => {
            let p = percentile(&finite, vec![lo, hi]);
            Some(GroupRange { min: p[0], max: p[1] })
        }
    }
}

/// Scale one value onto `[0, 1]`. Non-finite → `NaN`. Degenerate range → `0.5`.
pub fn scale_one(x: f64, range: GroupRange) -> f64 {
    if !x.is_finite() {
        return f64::NAN;
    }
    if range.is_degenerate() {
        return 0.5;
    }
    ((x - range.min) / (range.max - range.min)).clamp(0.0, 1.0)
}

/// Errors from grouped range fitting and relative k-means discretization.
#[derive(Debug, Clone, PartialEq)]
pub enum DiscretizeError {
    /// No samples.
    EmptyInput,
    /// `n_bins < 2`.
    TooFewBins {
        /// Requested bin count.
        n_bins: usize,
    },
    /// `values` and `group_ids` lengths differ.
    LengthMismatch {
        /// Length of `values`.
        n_values: usize,
        /// Length of `group_ids`.
        n_groups: usize,
    },
    /// Percentile range is not `0 ≤ lo < hi ≤ 100`.
    InvalidPercentileRange {
        /// Requested lower percentile.
        lo: f64,
        /// Requested upper percentile.
        hi: f64,
    },
    /// Not enough finite scaled points to place `n_bins` centroids.
    TooFewFinite {
        /// Finite pooled count.
        n_finite: usize,
        /// Requested bins.
        n_bins: usize,
    },
    /// Every group is constant (or no finite samples), so k-means has no spread.
    AllGroupsDegenerate,
    /// `transform` saw a `group_id` that was not present at fit.
    UnknownGroup {
        /// The unknown id.
        group: usize,
    },
}

impl fmt::Display for DiscretizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiscretizeError::EmptyInput => write!(f, "discretize: empty input"),
            DiscretizeError::TooFewBins { n_bins } => {
                write!(f, "discretize: n_bins must be ≥ 2, got {n_bins}")
            }
            DiscretizeError::LengthMismatch { n_values, n_groups } => {
                write!(f, "discretize: values length {n_values} != group_ids length {n_groups}")
            }
            DiscretizeError::InvalidPercentileRange { lo, hi } => {
                write!(
                    f,
                    "discretize: percentile range must satisfy 0 ≤ lo < hi ≤ 100, got lo={lo} hi={hi}"
                )
            }
            DiscretizeError::TooFewFinite { n_finite, n_bins } => {
                write!(f, "discretize: {n_finite} finite samples < n_bins {n_bins}")
            }
            DiscretizeError::AllGroupsDegenerate => {
                write!(f, "discretize: all groups have zero spread on the unit interval")
            }
            DiscretizeError::UnknownGroup { group } => {
                write!(f, "discretize: unknown group id {group} (not a silent fallback)")
            }
        }
    }
}

impl std::error::Error for DiscretizeError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> KMeansConfig {
        KMeansConfig {
            k: 3,
            seed: 1,
            ..KMeansConfig::default()
        }
    }

    /// Two users, same relative shape, different raw offset/scale.
    fn shifted_users() -> (Vec<f64>, Vec<usize>) {
        let a: Vec<f64> = (0..12).map(|i| i as f64).collect();
        let b: Vec<f64> = a.iter().map(|v| 100.0 + 2.0 * v).collect();
        let mut values = a;
        values.extend(b);
        let mut groups = vec![0usize; 12];
        groups.extend(vec![1usize; 12]);
        (values, groups)
    }

    #[test]
    fn shifted_users_share_relative_bins() {
        let (values, groups) = shifted_users();
        let disc = RelativeKMeansDiscretizer::fit(&values, &groups, 3, &cfg()).unwrap();
        let bins = disc.transform(&values, &groups).unwrap();
        assert_eq!(&bins[..12], &bins[12..]);
        assert!(disc.cuts.len() == 2);
        assert!(disc.cuts.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn constant_user_maps_to_mid_scale() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 9.0, 9.0, 9.0, 9.0, 9.0];
        let groups = vec![0, 0, 0, 0, 0, 1, 1, 1, 1, 1];
        let disc = RelativeKMeansDiscretizer::fit(&values, &groups, 3, &cfg()).unwrap();
        let bins = disc.transform(&values, &groups).unwrap();
        let const_bins = &bins[5..];
        assert!(const_bins.iter().all(|&b| b == const_bins[0]));
        let scaled = disc.scaler.transform(&[9.0], &[1]).unwrap();
        assert!((scaled[0] - 0.5).abs() < 1e-12);
    }

    #[test]
    fn transform_uses_frozen_train_range() {
        let values = vec![0.0, 1.0, 2.0, 3.0, 10.0, 11.0, 12.0, 13.0];
        let groups = vec![0, 0, 0, 0, 1, 1, 1, 1];
        let disc = RelativeKMeansDiscretizer::fit(&values, &groups, 3, &cfg()).unwrap();
        let bins_train = disc.transform(&values[..4], &groups[..4]).unwrap();
        // Interior points still use the train min/max of [0, 3], not a re-fit on [1, 2].
        let mid = disc.transform(&[1.0, 2.0], &[0, 0]).unwrap();
        assert_eq!(mid, bins_train[1..3]);
        let refit = disc.transform_new_group(&[1.0, 2.0]);
        assert_ne!(refit, mid);
    }

    #[test]
    fn transform_new_group_uses_own_range() {
        let (values, groups) = shifted_users();
        let disc = RelativeKMeansDiscretizer::fit(&values, &groups, 3, &cfg()).unwrap();
        let user_a = &values[..12];
        let bins_a = disc.transform(user_a, &groups[..12]).unwrap();

        let shifted: Vec<f64> = user_a.iter().map(|v| v + 500.0).collect();
        let bins_new = disc.transform_new_group(&shifted);
        assert_eq!(bins_a, bins_new);
    }

    #[test]
    fn percentile_range_ignores_spike() {
        let mut values: Vec<f64> = (0..40).map(|i| i as f64).collect();
        values[39] = 10_000.0;
        let groups = vec![0usize; 40];
        let mm = GroupedRangeScaler::fit(&values, &groups, RangeMethod::MinMax).unwrap();
        let pc = GroupedRangeScaler::fit(&values, &groups, RangeMethod::Percentile { lo: 5.0, hi: 95.0 }).unwrap();
        let r_mm = mm.range_of(0).unwrap();
        let r_pc = pc.range_of(0).unwrap();
        assert!(r_mm.max > 1_000.0);
        assert!(r_pc.max < 50.0);
    }

    #[test]
    fn unknown_group_is_error() {
        let (values, groups) = shifted_users();
        let disc = RelativeKMeansDiscretizer::fit(&values, &groups, 3, &cfg()).unwrap();
        let err = disc.transform(&[1.0], &[99]).unwrap_err();
        assert!(matches!(err, DiscretizeError::UnknownGroup { group: 99 }));
    }

    #[test]
    fn too_few_bins_errors() {
        let (values, groups) = shifted_users();
        let err = RelativeKMeansDiscretizer::fit(&values, &groups, 1, &cfg()).unwrap_err();
        assert!(matches!(err, DiscretizeError::TooFewBins { n_bins: 1 }));
    }

    #[test]
    fn length_mismatch_errors() {
        let err = GroupedRangeScaler::fit(&[1.0, 2.0], &[0], RangeMethod::MinMax).unwrap_err();
        assert!(matches!(err, DiscretizeError::LengthMismatch { .. }));
    }

    #[test]
    fn all_constant_errors() {
        let values = vec![3.0; 10];
        let groups = vec![0usize; 10];
        let err = RelativeKMeansDiscretizer::fit(&values, &groups, 3, &cfg()).unwrap_err();
        assert!(matches!(err, DiscretizeError::AllGroupsDegenerate));
    }
}
