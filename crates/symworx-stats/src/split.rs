// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Index-based train/test (and optional training-fold) partition plans.
//!
//! This module never copies or mutates the caller's data. It returns row
//! indices into `0..n` that can later be applied to slices, `ndarray`s,
//! DataFrames, or any aligned multi-column table.
//!
//! ## Minimum split size
//!
//! Each part must satisfy **both**:
//!
//! 1. **Absolute floor** — at least [`MIN_SPLIT_SAMPLES`] rows (10), so no
//!    part is absurdly small.
//! 2. **Parent-relative** — at least [`MIN_SPLIT_FRACTION`] of its parent:
//!
//! | Part | Parent |
//! |------|--------|
//! | Outer train / test | full dataset `n` |
//! | Each training fold | training set `n_train` |
//!
//! ```text
//! min_size(parent) = max(MIN_SPLIT_SAMPLES, ceil(MIN_SPLIT_FRACTION * parent))
//! ```
//!
//! Folds use `n_train` as parent (not total `n`). Equal 10-fold CV needs
//! `n_train ≥ 100` so each fold has ≥ 10 samples. On smaller trains,
//! [`max_train_folds`] reports how many folds fit.
//!
//! If a requested fold count is too large, [`train_test_split`] errors with
//! the maximum valid fold count for that train size.
//!
//! Fold sizes are **balanced**: for any fold partition,  
//! `max(fold lens) − min(fold lens) ≤ 1` (remainder assigned to earlier folds).
//!
//! ## Repeated resplits
//!
//! To estimate split-to-split variability, call [`repeated_train_test_split`]
//! (or loop [`train_test_split`] with different seeds). Each repeat is an
//! independent shuffle of the same outer ratio and fold count.

use std::fmt;

/// Minimum allowed size of a part as a fraction of its **parent** set.
///
/// - Outer train/test → parent is the full dataset (`n`).
/// - Training folds → parent is the training set (`n_train`).
pub const MIN_SPLIT_FRACTION: f64 = 0.10;

/// Absolute minimum number of samples in any split part (train, test, or fold).
///
/// Hard floor so parts never become tiny even when 10% of a small parent would
/// allow fewer than this many rows.
pub const MIN_SPLIT_SAMPLES: usize = 10;

/// Configuration for [`train_test_split`].
#[derive(Debug, Clone)]
pub struct SplitConfig {
    /// Fraction of rows assigned to the test set, in `(0, 1)`.
    ///
    /// Default: `0.3` (30% test / 70% train).
    pub test_ratio: f64,
    /// Optional number of folds over the **training** indices only.
    ///
    /// `None` → no CV folds. When `Some(k)`, `k` must be ≥ 2 and each fold
    /// must satisfy [`min_split_size`]`(n_train)` (≥ 10 samples and ≥ 10% of
    /// `n_train`).
    pub n_train_folds: Option<usize>,
    /// If `true`, shuffle indices before splitting (deterministic via `seed`).
    pub shuffle: bool,
    /// LCG seed used when `shuffle` is `true`.
    pub seed: u64,
}

impl Default for SplitConfig {
    fn default() -> Self {
        Self {
            test_ratio: 0.3,
            n_train_folds: None,
            shuffle: true,
            seed: 42,
        }
    }
}

/// Partition plan: indices into the original row space `0..n`.
///
/// Apply with [`take_indices`] / [`take_indices_cloned`], or with your
/// DataFrame / array gather of choice. Fold indices (when present) are also
/// in **original** coordinates, not train-relative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainTestSplit {
    /// Total number of rows the plan was built for.
    pub n: usize,
    /// Training-row indices (original space).
    pub train_idx: Vec<usize>,
    /// Test-row indices (original space).
    pub test_idx: Vec<usize>,
    /// Optional k-fold partition of the training set (original-space indices).
    /// Folds are disjoint, cover `train_idx` exactly, and are as equal in
    /// size as possible (remainder rows go to the earlier folds).
    pub folds: Option<Vec<Vec<usize>>>,
    /// Whether indices were shuffled before the train/test cut.
    pub shuffled: bool,
    /// Seed used for shuffling, if any shuffle occurred.
    pub seed: Option<u64>,
}

impl TrainTestSplit {
    /// Validation-fold indices for fold `fold` (original space).
    ///
    /// Returns `None` if there are no folds or `fold` is out of range.
    pub fn val_idx(&self, fold: usize) -> Option<&[usize]> {
        self.folds.as_ref()?.get(fold).map(|v| v.as_slice())
    }

    /// Fit-set indices for fold `fold`: all training rows except that fold.
    ///
    /// Returns `None` if there are no folds or `fold` is out of range.
    pub fn fit_idx(&self, fold: usize) -> Option<Vec<usize>> {
        let folds = self.folds.as_ref()?;
        if fold >= folds.len() {
            return None;
        }
        let mut out = Vec::with_capacity(self.train_idx.len().saturating_sub(folds[fold].len()));
        for (i, f) in folds.iter().enumerate() {
            if i != fold {
                out.extend_from_slice(f);
            }
        }
        Some(out)
    }

    /// Number of training folds, or `0` if folds were not requested.
    pub fn n_folds(&self) -> usize {
        self.folds.as_ref().map_or(0, |f| f.len())
    }
}

/// Which part of a split failed the minimum-size check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitPart {
    /// Outer test hold-out (parent = full dataset).
    Test,
    /// Outer training set before folding (parent = full dataset).
    Train,
    /// One of the k training folds (parent = training set).
    TrainFold,
}

impl fmt::Display for SplitPart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SplitPart::Test => write!(f, "test"),
            SplitPart::Train => write!(f, "train"),
            SplitPart::TrainFold => write!(f, "train fold"),
        }
    }
}

/// Errors from [`train_test_split`] and related helpers.
#[derive(Debug, Clone, PartialEq)]
pub enum SplitError {
    /// `n == 0`.
    EmptyDataset,
    /// `test_ratio` not strictly inside `(0, 1)`.
    InvalidTestRatio {
        /// The invalid ratio.
        test_ratio: f64,
    },
    /// Requested fold count is not usable (`0`, `1`, or greater than train size).
    InvalidFoldCount {
        /// Requested number of folds.
        n_folds: usize,
        /// Training set size for context.
        n_train: usize,
    },
    /// A resulting part is smaller than [`min_split_size`] of its parent.
    SplitTooSmall {
        /// Which part failed the check.
        part: SplitPart,
        /// Actual size of that part.
        size: usize,
        /// Minimum allowed size
        /// (`max(MIN_SPLIT_SAMPLES, ceil(MIN_SPLIT_FRACTION * parent))`).
        min_size: usize,
        /// Full dataset size (`n`).
        n: usize,
        /// Parent size used for the threshold (`n` for outer parts, `n_train`
        /// for folds).
        parent: usize,
        /// When folds are involved, the largest valid `n_train_folds`;
        /// otherwise `None`.
        max_folds: Option<usize>,
    },
    /// Rounding left train or test empty (very small `n` or extreme ratio).
    EmptyPart {
        /// Which part is empty.
        part: SplitPart,
        /// Total dataset size.
        n: usize,
    },
    /// `n_repeats` was zero in [`repeated_train_test_split`].
    InvalidRepeatCount {
        /// Requested number of repeats.
        n_repeats: usize,
    },
}

impl fmt::Display for SplitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SplitError::EmptyDataset => write!(f, "cannot split an empty dataset (n = 0)"),
            SplitError::InvalidTestRatio { test_ratio } => {
                write!(f, "test_ratio must be in (0, 1), got {test_ratio}")
            }
            SplitError::InvalidFoldCount { n_folds, n_train } => write!(
                f,
                "n_train_folds must be in 2..=n_train (n_train = {n_train}), got {n_folds}"
            ),
            SplitError::SplitTooSmall {
                part,
                size,
                min_size,
                n,
                parent,
                max_folds,
            } => {
                let parent_label = match part {
                    SplitPart::TrainFold => "training set",
                    SplitPart::Test | SplitPart::Train => "dataset",
                };
                write!(
                    f,
                    "{part} size {size} is below the minimum of {min_size} \
                     (need ≥ {MIN_SPLIT_SAMPLES} samples and ≥ {:.0}% of the \
                     {parent_label}; parent = {parent}, n = {n})",
                    MIN_SPLIT_FRACTION * 100.0
                )?;
                if let Some(max_k) = max_folds {
                    write!(f, "; maximum number of training folds for this split: {max_k}")?;
                }
                Ok(())
            }
            SplitError::EmptyPart { part, n } => {
                write!(f, "{part} set is empty after sizing for n = {n}")
            }
            SplitError::InvalidRepeatCount { n_repeats } => {
                write!(f, "n_repeats must be ≥ 1, got {n_repeats}")
            }
        }
    }
}

impl std::error::Error for SplitError {}

/// Minimum number of rows a part may have when its parent has size `parent`.
///
/// ```text
/// max(MIN_SPLIT_SAMPLES, ceil(MIN_SPLIT_FRACTION * parent))
/// ```
///
/// Returns `0` when `parent == 0`. Call with `parent = n` for outer train/test,
/// or `parent = n_train` for folds.
#[inline]
pub fn min_split_size(parent: usize) -> usize {
    if parent == 0 {
        return 0;
    }
    let frac = (parent as f64 * MIN_SPLIT_FRACTION).ceil() as usize;
    frac.max(MIN_SPLIT_SAMPLES)
}

/// Largest number of balanced training folds allowed under the fold rule
/// for the given outer `n` and `test_ratio`.
///
/// Each fold must be ≥ [`min_split_size`]`(n_train)`. Returns `0` if the outer
/// train/test split is invalid or if fewer than 2 folds would fit.
pub fn max_train_folds(n: usize, test_ratio: f64) -> usize {
    if n == 0 || !(test_ratio > 0.0 && test_ratio < 1.0) {
        return 0;
    }
    let (n_train, n_test) = match train_test_sizes(n, test_ratio) {
        Ok(sizes) => sizes,
        Err(_) => return 0,
    };
    // Outer parts must still clear 10% of total n.
    let min_outer = min_split_size(n);
    if n_test < min_outer || n_train < min_outer {
        return 0;
    }
    max_train_folds_from_train(n_train)
}

/// Number of train and test rows for `n` and `test_ratio`.
///
/// Uses rounding for the test count, then clamps so both sides are non-empty
/// when `n >= 2`.
fn train_test_sizes(n: usize, test_ratio: f64) -> Result<(usize, usize), SplitError> {
    if n == 0 {
        return Err(SplitError::EmptyDataset);
    }
    if !(test_ratio > 0.0 && test_ratio < 1.0) {
        return Err(SplitError::InvalidTestRatio { test_ratio });
    }
    if n == 1 {
        // Cannot form both a non-empty train and test.
        return Err(SplitError::EmptyPart {
            part: SplitPart::Test,
            n,
        });
    }

    let mut n_test = (test_ratio * n as f64).round() as usize;
    if n_test == 0 {
        n_test = 1;
    }
    if n_test >= n {
        n_test = n - 1;
    }
    let n_train = n - n_test;
    if n_train == 0 {
        return Err(SplitError::EmptyPart {
            part: SplitPart::Train,
            n,
        });
    }
    if n_test == 0 {
        return Err(SplitError::EmptyPart {
            part: SplitPart::Test,
            n,
        });
    }
    Ok((n_train, n_test))
}

/// Build an index-only train/test partition plan (optional training folds).
///
/// # Arguments
/// * `n` — number of rows / samples in the dataset
/// * `config` — ratios, folds, shuffle
///
/// # Errors
/// See [`SplitError`]. Outer train/test must each meet [`min_split_size`]`(n)`.
/// Each training fold must meet [`min_split_size`]`(n_train)`. Fold failures
/// include a `max_folds` hint.
///
/// # Example
/// ```
/// use symworx_stats::split::{train_test_split, SplitConfig};
///
/// // n_train = 140 → 10 folds of 14 ≥ absolute floor of 10
/// let split = train_test_split(
///     200,
///     &SplitConfig {
///         test_ratio: 0.3,
///         n_train_folds: Some(10),
///         shuffle: true,
///         seed: 7,
///     },
/// )
/// .unwrap();
///
/// assert_eq!(split.train_idx.len() + split.test_idx.len(), 200);
/// assert_eq!(split.n_folds(), 10);
/// // Apply later: take_indices(&rows, &split.train_idx)
/// ```
pub fn train_test_split(n: usize, config: &SplitConfig) -> Result<TrainTestSplit, SplitError> {
    let (n_train, n_test) = train_test_sizes(n, config.test_ratio)?;
    let min_outer = min_split_size(n);

    if n_test < min_outer {
        return Err(SplitError::SplitTooSmall {
            part: SplitPart::Test,
            size: n_test,
            min_size: min_outer,
            n,
            parent: n,
            max_folds: None,
        });
    }
    if n_train < min_outer {
        return Err(SplitError::SplitTooSmall {
            part: SplitPart::Train,
            size: n_train,
            min_size: min_outer,
            n,
            parent: n,
            max_folds: None,
        });
    }

    let mut indices: Vec<usize> = (0..n).collect();
    let seed_used = if config.shuffle {
        let mut state = config.seed;
        fisher_yates_shuffle(&mut indices, &mut state);
        Some(config.seed)
    } else {
        None
    };

    // Contiguous cut after optional shuffle: first n_train → train, rest → test.
    let train_idx: Vec<usize> = indices[..n_train].to_vec();
    let test_idx: Vec<usize> = indices[n_train..].to_vec();

    let folds = if let Some(k) = config.n_train_folds {
        Some(build_train_folds(&train_idx, k, n)?)
    } else {
        None
    };

    Ok(TrainTestSplit {
        n,
        train_idx,
        test_idx,
        folds,
        shuffled: config.shuffle,
        seed: seed_used,
    })
}

/// Build `n_repeats` independent partition plans with the same ratios/folds.
///
/// Each repeat uses `config.seed.wrapping_add(repeat_index as u64)` so results
/// are deterministic and distinct when `shuffle` is `true`. If `shuffle` is
/// `false`, every repeat is identical (same contiguous cut) — prefer
/// `shuffle: true` for meaningful resampling.
///
/// # Validity
///
/// Repeated outer hold-outs (with or without training folds each time) are a
/// standard way to assess sensitivity to the random split. They are **not**
/// a substitute for a single untouched final test set if you later tune on
/// the repeated results; treat aggregate metrics as exploratory or use an
/// outer lockbox hold-out for final reporting.
///
/// # Errors
///
/// Same as [`train_test_split`], or [`SplitError::InvalidRepeatCount`] when
/// `n_repeats == 0`.
///
/// # Example
/// ```
/// use symworx_stats::split::{repeated_train_test_split, SplitConfig};
///
/// let plans = repeated_train_test_split(
///     200,
///     &SplitConfig {
///         test_ratio: 0.3,
///         n_train_folds: Some(5),
///         shuffle: true,
///         seed: 42,
///     },
///     5,
/// )
/// .unwrap();
/// assert_eq!(plans.len(), 5);
/// assert_ne!(plans[0].train_idx, plans[1].train_idx);
/// ```
pub fn repeated_train_test_split(
    n: usize,
    config: &SplitConfig,
    n_repeats: usize,
) -> Result<Vec<TrainTestSplit>, SplitError> {
    if n_repeats == 0 {
        return Err(SplitError::InvalidRepeatCount { n_repeats: 0 });
    }
    let mut plans = Vec::with_capacity(n_repeats);
    for r in 0..n_repeats {
        let mut cfg = config.clone();
        cfg.seed = config.seed.wrapping_add(r as u64);
        plans.push(train_test_split(n, &cfg)?);
    }
    Ok(plans)
}

/// Partition `train_idx` into `k` folds in original index space.
///
/// Fold sizes must meet [`min_split_size`]`(n_train)`. Guarantees
/// `max(len) − min(len) ≤ 1` across folds.
fn build_train_folds(train_idx: &[usize], k: usize, n: usize) -> Result<Vec<Vec<usize>>, SplitError> {
    let n_train = train_idx.len();
    if k < 2 || k > n_train {
        return Err(SplitError::InvalidFoldCount { n_folds: k, n_train });
    }

    let min_fold_req = min_split_size(n_train);
    // Smallest fold under balanced split is floor(n_train / k).
    let min_fold = n_train / k;
    if min_fold < min_fold_req {
        let max_k = max_train_folds_from_train(n_train);
        return Err(SplitError::SplitTooSmall {
            part: SplitPart::TrainFold,
            size: min_fold,
            min_size: min_fold_req,
            n,
            parent: n_train,
            max_folds: Some(max_k),
        });
    }

    // Balanced sizes: first `rem` folds get base+1, the rest get base.
    // Invariant: max(len) - min(len) <= 1.
    let base = n_train / k;
    let rem = n_train % k;
    let mut folds = Vec::with_capacity(k);
    let mut offset = 0;
    for i in 0..k {
        let len = base + usize::from(i < rem);
        folds.push(train_idx[offset..offset + len].to_vec());
        offset += len;
    }
    debug_assert_eq!(offset, n_train);
    debug_assert!(
        folds_balanced(&folds),
        "fold sizes must satisfy max - min <= 1, got {:?}",
        folds.iter().map(|f| f.len()).collect::<Vec<_>>()
    );
    Ok(folds)
}

/// `true` if fold lengths differ by at most one (or there are no folds).
#[inline]
fn folds_balanced(folds: &[Vec<usize>]) -> bool {
    if folds.is_empty() {
        return true;
    }
    let mut min_l = usize::MAX;
    let mut max_l = 0usize;
    for f in folds {
        min_l = min_l.min(f.len());
        max_l = max_l.max(f.len());
    }
    max_l - min_l <= 1
}

/// Max k with floor(n_train / k) ≥ min_split_size(n_train).
#[inline]
fn max_train_folds_from_train(n_train: usize) -> usize {
    let min_sz = min_split_size(n_train);
    if min_sz == 0 {
        return n_train;
    }
    let max_k = n_train / min_sz;
    if max_k < 2 { 0 } else { max_k.min(n_train) }
}

/// Gather references into `data` by row index (no clone of elements).
///
/// # Panics
/// Panics if any index is out of bounds for `data`.
pub fn take_indices<'a, T>(data: &'a [T], indices: &[usize]) -> Vec<&'a T> {
    indices.iter().map(|&i| &data[i]).collect()
}

/// Gather owned clones of `data` by row index.
///
/// # Panics
/// Panics if any index is out of bounds for `data`.
pub fn take_indices_cloned<T: Clone>(data: &[T], indices: &[usize]) -> Vec<T> {
    indices.iter().map(|&i| data[i].clone()).collect()
}

/// Fisher–Yates shuffle using the same LCG style as `cluster` (no `rand` dep).
fn fisher_yates_shuffle(indices: &mut [usize], state: &mut u64) {
    let n = indices.len();
    if n < 2 {
        return;
    }
    for i in (1..n).rev() {
        let j = (lcg_next(state) as usize) % (i + 1);
        indices.swap(i, j);
    }
}

/// Simple LCG for deterministic, dependency-free seeding (Numerical Recipes).
fn lcg_next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(1664525).wrapping_add(1013904223);
    *state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_split_size_basic() {
        assert_eq!(min_split_size(0), 0);
        // Absolute floor dominates on small parents
        assert_eq!(min_split_size(15), MIN_SPLIT_SAMPLES); // max(10, 2)
        assert_eq!(min_split_size(70), MIN_SPLIT_SAMPLES); // max(10, 7)
        assert_eq!(min_split_size(100), 10); // max(10, 10)
        // Fraction dominates on large parents
        assert_eq!(min_split_size(200), 20); // max(10, 20)
        assert_eq!(min_split_size(700), 70); // max(10, 70)
    }

    #[test]
    fn train_test_70_30_no_folds() {
        let split = train_test_split(
            100,
            &SplitConfig {
                test_ratio: 0.3,
                n_train_folds: None,
                shuffle: false,
                seed: 0,
            },
        )
        .unwrap();

        assert_eq!(split.n, 100);
        assert_eq!(split.train_idx.len(), 70);
        assert_eq!(split.test_idx.len(), 30);
        assert!(split.folds.is_none());
        // No shuffle: train is 0..70, test is 70..100
        assert_eq!(split.train_idx, (0..70).collect::<Vec<_>>());
        assert_eq!(split.test_idx, (70..100).collect::<Vec<_>>());
    }

    #[test]
    fn folds_five_ok_on_100() {
        // train=70, fold min = max(10, 7)=10; 5 folds → size 14 ≥ 10
        let split = train_test_split(
            100,
            &SplitConfig {
                test_ratio: 0.3,
                n_train_folds: Some(5),
                shuffle: false,
                seed: 0,
            },
        )
        .unwrap();

        assert_eq!(split.n_folds(), 5);
        let folds = split.folds.as_ref().unwrap();
        let total: usize = folds.iter().map(|f| f.len()).sum();
        assert_eq!(total, 70);
        for f in folds {
            assert!(f.len() >= min_split_size(70));
        }

        let val = split.val_idx(0).unwrap();
        let fit = split.fit_idx(0).unwrap();
        assert_eq!(val.len() + fit.len(), 70);
    }

    #[test]
    fn ten_folds_on_100_errors_absolute_floor() {
        // fold size 7 < MIN_SPLIT_SAMPLES (10); max k = floor(70/10) = 7
        let err = train_test_split(
            100,
            &SplitConfig {
                test_ratio: 0.3,
                n_train_folds: Some(10),
                shuffle: false,
                seed: 0,
            },
        )
        .unwrap_err();

        match err {
            SplitError::SplitTooSmall {
                part: SplitPart::TrainFold,
                size: 7,
                min_size: 10,
                n: 100,
                parent: 70,
                max_folds: Some(7),
            } => {}
            other => panic!("unexpected error: {other:?}"),
        }
        assert_eq!(max_train_folds(100, 0.3), 7);
        assert!(err.to_string().contains("maximum number of training folds"));
    }

    #[test]
    fn ten_folds_on_1000_ok() {
        // train=700, fold min = max(10, 70)=70; 10 folds of 70 ≥ 70
        let split = train_test_split(
            1000,
            &SplitConfig {
                test_ratio: 0.3,
                n_train_folds: Some(10),
                shuffle: true,
                seed: 1,
            },
        )
        .unwrap();
        assert_eq!(split.train_idx.len(), 700);
        assert_eq!(split.test_idx.len(), 300);
        assert_eq!(split.n_folds(), 10);
        assert_eq!(max_train_folds(1000, 0.3), 10);
    }

    #[test]
    fn ten_folds_needs_enough_train() {
        // n=200 → train=140, fold min = max(10, 14)=14; 10 folds of 14 OK
        let split = train_test_split(
            200,
            &SplitConfig {
                test_ratio: 0.3,
                n_train_folds: Some(10),
                shuffle: false,
                seed: 0,
            },
        )
        .unwrap();
        assert_eq!(split.n_folds(), 10);
        for f in split.folds.as_ref().unwrap() {
            assert!(f.len() >= MIN_SPLIT_SAMPLES);
        }
    }

    #[test]
    fn eleven_folds_errors_with_max() {
        // n=100, train=70: floor(70/11)=6 < 10 → max k = 7
        let err = train_test_split(
            100,
            &SplitConfig {
                test_ratio: 0.3,
                n_train_folds: Some(11),
                shuffle: false,
                seed: 0,
            },
        )
        .unwrap_err();

        match err {
            SplitError::SplitTooSmall {
                part: SplitPart::TrainFold,
                size: 6,
                min_size: 10,
                n: 100,
                parent: 70,
                max_folds: Some(7),
            } => {}
            other => panic!("unexpected error: {other:?}"),
        }

        let msg = err.to_string();
        assert!(msg.contains("training set"));
        assert!(msg.contains("maximum number of training folds"));
    }

    #[test]
    fn tiny_test_ratio_errors() {
        // 5% test of 100 → 5 < 10% of total n
        let err = train_test_split(
            100,
            &SplitConfig {
                test_ratio: 0.05,
                n_train_folds: None,
                shuffle: false,
                seed: 0,
            },
        )
        .unwrap_err();

        match err {
            SplitError::SplitTooSmall {
                part: SplitPart::Test,
                size: 5,
                min_size: 10,
                n: 100,
                parent: 100,
                max_folds: None,
            } => {}
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn shuffle_is_deterministic() {
        let cfg = SplitConfig {
            test_ratio: 0.25,
            n_train_folds: Some(4),
            shuffle: true,
            seed: 12345,
        };
        let a = train_test_split(80, &cfg).unwrap();
        let b = train_test_split(80, &cfg).unwrap();
        assert_eq!(a, b);
        assert_ne!(a.train_idx, (0..60).collect::<Vec<_>>());
    }

    #[test]
    fn indices_partition_uniquely() {
        let split = train_test_split(
            50,
            &SplitConfig {
                test_ratio: 0.2,
                n_train_folds: Some(4),
                shuffle: true,
                seed: 9,
            },
        )
        .unwrap();

        let mut all = split.train_idx.clone();
        all.extend_from_slice(&split.test_idx);
        all.sort_unstable();
        assert_eq!(all, (0..50).collect::<Vec<_>>());

        let folds = split.folds.as_ref().unwrap();
        let mut from_folds: Vec<usize> = folds.iter().flatten().copied().collect();
        from_folds.sort_unstable();
        let mut train_sorted = split.train_idx.clone();
        train_sorted.sort_unstable();
        assert_eq!(from_folds, train_sorted);
    }

    #[test]
    fn take_indices_works() {
        let data: Vec<String> = (0..50).map(|i| format!("r{i}")).collect();
        let split = train_test_split(
            data.len(),
            &SplitConfig {
                test_ratio: 0.3,
                n_train_folds: None,
                shuffle: false,
                seed: 0,
            },
        )
        .unwrap();
        // n=50 → train=35, test=15; both ≥ 10
        assert_eq!(split.train_idx.len(), 35);
        assert_eq!(split.test_idx.len(), 15);
        let train = take_indices_cloned(&data, &split.train_idx);
        let test = take_indices_cloned(&data, &split.test_idx);
        assert_eq!(train[0], "r0");
        assert_eq!(test[0], "r35");
        assert_eq!(train.len() + test.len(), 50);
    }

    #[test]
    fn absolute_floor_rejects_tiny_dataset() {
        // n=15 cannot form train and test both ≥ 10
        let err = train_test_split(
            15,
            &SplitConfig {
                test_ratio: 0.3,
                n_train_folds: None,
                shuffle: false,
                seed: 0,
            },
        )
        .unwrap_err();
        assert!(matches!(
            err,
            SplitError::SplitTooSmall {
                part: SplitPart::Test | SplitPart::Train,
                ..
            }
        ));
    }

    #[test]
    fn invalid_ratio_and_empty() {
        assert!(matches!(
            train_test_split(
                10,
                &SplitConfig {
                    test_ratio: 0.0,
                    ..Default::default()
                }
            ),
            Err(SplitError::InvalidTestRatio { .. })
        ));
        assert!(matches!(
            train_test_split(0, &SplitConfig::default()),
            Err(SplitError::EmptyDataset)
        ));
    }

    #[test]
    fn invalid_fold_count() {
        let err = train_test_split(
            100,
            &SplitConfig {
                test_ratio: 0.3,
                n_train_folds: Some(1),
                shuffle: false,
                seed: 0,
            },
        )
        .unwrap_err();
        assert!(matches!(err, SplitError::InvalidFoldCount { n_folds: 1, .. }));
    }

    #[test]
    fn fold_sizes_max_minus_min_at_most_one() {
        // Odd n and n_train so rem != 0
        for n in [101usize, 103, 200, 997] {
            let split = train_test_split(
                n,
                &SplitConfig {
                    test_ratio: 0.3,
                    n_train_folds: Some(5),
                    shuffle: true,
                    seed: 99,
                },
            )
            .unwrap();
            let folds = split.folds.as_ref().unwrap();
            assert!(
                folds_balanced(folds),
                "n={n}: sizes {:?}",
                folds.iter().map(|f| f.len()).collect::<Vec<_>>()
            );
            let total: usize = folds.iter().map(|f| f.len()).sum();
            assert_eq!(total, split.train_idx.len());
        }
    }

    #[test]
    fn n101_default_sizes() {
        let split = train_test_split(
            101,
            &SplitConfig {
                test_ratio: 0.3,
                n_train_folds: Some(5),
                shuffle: false,
                seed: 0,
            },
        )
        .unwrap();
        assert_eq!(split.train_idx.len(), 71);
        assert_eq!(split.test_idx.len(), 30);
        let sizes: Vec<usize> = split.folds.as_ref().unwrap().iter().map(|f| f.len()).collect();
        assert_eq!(sizes, vec![15, 14, 14, 14, 14]);
        assert!(folds_balanced(split.folds.as_ref().unwrap()));
    }

    #[test]
    fn repeated_splits_are_distinct_with_shuffle() {
        let plans = repeated_train_test_split(
            200,
            &SplitConfig {
                test_ratio: 0.3,
                n_train_folds: Some(5),
                shuffle: true,
                seed: 42,
            },
            5,
        )
        .unwrap();
        assert_eq!(plans.len(), 5);
        // Different seeds → different shuffles
        assert_ne!(plans[0].train_idx, plans[1].train_idx);
        assert_ne!(plans[0].test_idx, plans[2].test_idx);
        // Same outer counts every time
        for p in &plans {
            assert_eq!(p.train_idx.len() + p.test_idx.len(), 200);
            assert_eq!(p.n_folds(), 5);
            assert!(folds_balanced(p.folds.as_ref().unwrap()));
        }
        // Deterministic
        let again = repeated_train_test_split(
            200,
            &SplitConfig {
                test_ratio: 0.3,
                n_train_folds: Some(5),
                shuffle: true,
                seed: 42,
            },
            5,
        )
        .unwrap();
        assert_eq!(plans, again);
    }

    #[test]
    fn repeated_zero_errors() {
        let err = repeated_train_test_split(50, &SplitConfig::default(), 0).unwrap_err();
        assert!(matches!(err, SplitError::InvalidRepeatCount { n_repeats: 0 }));
    }
}
