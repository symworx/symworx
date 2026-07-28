// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Index-based train/test (and training-fold) partition plans.
//!
//! Demonstrates `symworx-stats::split`:
//! - Build a [`TrainTestSplit`] from row count only (data is never copied)
//! - Apply indices later with [`take_indices_cloned`]
//! - Optional k-fold partition of the **training** set
//! - Minimum size = max(10 samples, 10% of parent)
//!
//! Run with:
//! ```bash
//! cargo run -p symworx-stats --example train_test_split_demo
//! ```

use symworx_stats::{
    MIN_SPLIT_FRACTION,
    MIN_SPLIT_SAMPLES,
    SplitConfig,
    SplitError,
    max_train_folds,
    min_split_size,
    repeated_train_test_split,
    take_indices_cloned,
    train_test_split,
};

fn main() {
    println!("=== symworx-stats: train / test split demo ===\n");

    let n = 100;
    let sample_ids: Vec<usize> = (0..n).collect();
    let feature: Vec<f64> = (0..n).map(|i| (i as f64) * 0.1).collect();

    println!("Dataset: n = {n} rows (ids + one feature column)");
    println!(
        "Policy: min_size(parent) = max({MIN_SPLIT_SAMPLES}, ceil({:.0}% · parent))",
        MIN_SPLIT_FRACTION * 100.0
    );
    println!(
        "  outer parent = n → min {}\n  fold parent = n_train\n",
        min_split_size(n)
    );

    // --- 1) Outer train / test (70 / 30), no CV folds ---
    println!("1) Outer hold-out: test_ratio = 0.3, no training folds");
    let holdout = train_test_split(
        n,
        &SplitConfig {
            test_ratio: 0.3,
            n_train_folds: None,
            shuffle: true,
            seed: 42,
        },
    )
    .expect("70/30 is valid for n = 100");

    println!(
        "   train = {} rows, test = {} rows (shuffled = {})",
        holdout.train_idx.len(),
        holdout.test_idx.len(),
        holdout.shuffled
    );
    println!(
        "   first 8 train idx: {:?}",
        &holdout.train_idx[..holdout.train_idx.len().min(8)]
    );

    let train_ids = take_indices_cloned(&sample_ids, &holdout.train_idx);
    let test_x = take_indices_cloned(&feature, &holdout.test_idx);
    println!(
        "   applied: train_ids[0..3]={:?}  test_x[0..3]={:?}",
        &train_ids[..3],
        &test_x[..3]
    );
    println!("   original feature.len() still {} (untouched)\n", feature.len());

    // --- 2) 5 folds OK on n=100 (fold size 14 ≥ 10) ---
    println!("2) 70/30 + 5 training folds on n = 100");
    let max_k = max_train_folds(n, 0.3);
    println!(
        "   max_train_folds(100, 0.3) = {max_k}  (fold min = min_split_size(70) = {})",
        min_split_size(70)
    );

    let cv = train_test_split(
        n,
        &SplitConfig {
            test_ratio: 0.3,
            n_train_folds: Some(5),
            shuffle: true,
            seed: 42,
        },
    )
    .expect("5 folds is within max");

    for k in 0..cv.n_folds() {
        let val = cv.val_idx(k).unwrap();
        let fit = cv.fit_idx(k).unwrap();
        println!("   fold {k}: val={}, fit={}", val.len(), fit.len());
    }
    println!();

    // --- 3) 10 folds on n=100 fails absolute floor ---
    println!("3) Request 10 folds on n = 100 (fold size 7 < {MIN_SPLIT_SAMPLES})");
    match train_test_split(
        n,
        &SplitConfig {
            test_ratio: 0.3,
            n_train_folds: Some(10),
            shuffle: false,
            seed: 0,
        },
    ) {
        Ok(_) => println!("   unexpected success"),
        Err(e) => {
            println!("   error: {e}");
            if let SplitError::SplitTooSmall {
                size,
                min_size,
                max_folds,
                ..
            } = e
            {
                println!(
                    "   detail: fold size {size}, need ≥ {min_size}; max folds = {:?}",
                    max_folds
                );
            }
        }
    }

    // --- 4) Large enough train for 10-fold ---
    println!("\n4) n = 1000, 30% test, 10 folds (train=700, fold=70 ≥ 10 and ≥ 10% of train)");
    let large = train_test_split(
        1000,
        &SplitConfig {
            test_ratio: 0.3,
            n_train_folds: Some(10),
            shuffle: true,
            seed: 7,
        },
    )
    .expect("enough train samples for 10-fold");
    println!(
        "   train={}, test={}, n_folds={}, fold_len≈{}",
        large.train_idx.len(),
        large.test_idx.len(),
        large.n_folds(),
        large.val_idx(0).map(|v| v.len()).unwrap_or(0)
    );

    // --- 5) Tiny test ratio still rejected ---
    println!("\n5) test_ratio = 0.05 on n = 100 (test size 5 < {MIN_SPLIT_SAMPLES})");
    match train_test_split(
        n,
        &SplitConfig {
            test_ratio: 0.05,
            n_train_folds: None,
            shuffle: false,
            seed: 0,
        },
    ) {
        Ok(_) => println!("   unexpected success"),
        Err(e) => println!("   error: {e}"),
    }

    // --- 6) Repeated resplits (5 independent plans) ---
    println!("\n6) repeated_train_test_split: 5 resplits of n = 200 with 5 folds each");
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
    .expect("repeats");
    for (i, p) in plans.iter().enumerate() {
        let fold_sizes: Vec<usize> = p.folds.as_ref().unwrap().iter().map(|f| f.len()).collect();
        println!(
            "   repeat {i}: train={}, test={}, fold_sizes={fold_sizes:?}, seed={:?}",
            p.train_idx.len(),
            p.test_idx.len(),
            p.seed
        );
    }
    println!("   (same ratio/folds; different shuffles — use for split-variance checks)");

    println!("\nDone. Use the same TrainTestSplit indices on any table backend.");
}
