// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! ML-style binary logistic regression: split → fit → hold-out → k-fold CV.
//!
//! For a minimal fit/predict demo (no splits), see `logistic_regression_demo`.
//!
//! ```bash
//! cargo run -p symworx-stats --example logistic_ml_pipeline_demo
//! ```

use ndarray::{
    Array1,
    Array2,
};
use symworx_stats::{
    LogisticConfig,
    LogisticModel,
    SplitConfig,
    logistic_regression,
    max_train_folds,
    take_indices_cloned,
    train_test_split,
};

fn main() {
    println!("=== symworx-stats: logistic regression (ML pipeline) ===\n");

    // ------------------------------------------------------------------
    // 1) Dataset  —  class 1 when x0 + x1 > 1 (noisy boundary)
    // ------------------------------------------------------------------
    let (x, y) = make_classification(80, 42);
    let n = x.nrows();
    let n_pos = y.iter().filter(|&&v| v > 0.5).count();
    println!("1) Dataset");
    println!(
        "   n = {n}, n_features = {}, class balance: {} positive / {} negative",
        x.ncols(),
        n_pos,
        n - n_pos
    );

    // ------------------------------------------------------------------
    // 2) Index-only split plan (original X, y untouched)
    // ------------------------------------------------------------------
    let test_ratio = 0.3;
    let n_folds = 5;
    let max_k = max_train_folds(n, test_ratio);
    println!("\n2) Split plan (indices only)");
    println!("   test_ratio = {test_ratio}, n_train_folds = {n_folds} (max allowed = {max_k})");

    let plan = train_test_split(
        n,
        &SplitConfig {
            test_ratio,
            n_train_folds: Some(n_folds),
            shuffle: true,
            seed: 7,
        },
    )
    .expect("valid split for n = 80");

    println!(
        "   outer: train = {}, test = {}  (shuffled, seed = {:?})",
        plan.train_idx.len(),
        plan.test_idx.len(),
        plan.seed
    );
    for k in 0..plan.n_folds() {
        let val = plan.val_idx(k).unwrap();
        let fit = plan.fit_idx(k).unwrap();
        println!("   fold {k}: fit = {}, val = {}", fit.len(), val.len());
    }

    let x_train = rows_at(&x, &plan.train_idx);
    let y_train = array1_at(&y, &plan.train_idx);
    let x_test = rows_at(&x, &plan.test_idx);
    let y_test = array1_at(&y, &plan.test_idx);

    let cfg = LogisticConfig {
        max_iter: 8000,
        learning_rate: 0.35,
        l2: 0.01,
        tol: 1e-8,
        threshold: 0.5,
        ..Default::default()
    };

    // ------------------------------------------------------------------
    // 3) Fit on full training set → evaluate hold-out test
    // ------------------------------------------------------------------
    println!("\n3) Fit on training set → score hold-out test");
    let model = logistic_regression(&x_train, &y_train, &cfg);
    print_model(&model);

    let train_acc = model.accuracy(&x_train, &y_train, cfg.threshold);
    let test_acc = model.accuracy(&x_test, &y_test, cfg.threshold);
    println!("   train accuracy = {train_acc:.3}");
    println!("   test  accuracy = {test_acc:.3}");

    println!("\n   sample test predictions:");
    let p_test = model.predict_proba(&x_test);
    let yhat = model.predict(&x_test, cfg.threshold);
    for i in 0..y_test.len().min(6) {
        println!(
            "     y={:.0}  p(y=1)={:.3}  ŷ={:.0}",
            y_test[i], p_test[i], yhat[i]
        );
    }

    // ------------------------------------------------------------------
    // 4) K-fold CV on training indices only (test never seen)
    // ------------------------------------------------------------------
    println!("\n4) {n_folds}-fold CV on training set (hold-out test still locked away)");
    let mut fold_acc = Vec::with_capacity(n_folds);
    for k in 0..plan.n_folds() {
        let fit_idx = plan.fit_idx(k).unwrap();
        let val_idx = plan.val_idx(k).unwrap();
        let x_fit = rows_at(&x, &fit_idx);
        let y_fit = array1_at(&y, &fit_idx);
        let x_val = rows_at(&x, &val_idx);
        let y_val = array1_at(&y, &val_idx);

        let fold_model = logistic_regression(&x_fit, &y_fit, &cfg);
        let acc = fold_model.accuracy(&x_val, &y_val, cfg.threshold);
        fold_acc.push(acc);
        println!("   fold {k}: val accuracy = {acc:.3}");
    }
    let mean_cv = fold_acc.iter().sum::<f64>() / fold_acc.len() as f64;
    let var_cv =
        fold_acc.iter().map(|a| (a - mean_cv).powi(2)).sum::<f64>() / fold_acc.len() as f64;
    println!(
        "   CV mean accuracy = {mean_cv:.3}  (std ≈ {:.3})",
        var_cv.sqrt()
    );
    println!("   final hold-out test accuracy = {test_acc:.3}  (single frozen test)");

    println!("\nDone.");
    println!("  Simple fit/predict: cargo run -p symworx-stats --example logistic_regression_demo");
}

fn make_classification(n: usize, seed: u64) -> (Array2<f64>, Array1<f64>) {
    let mut state = seed;
    let mut x = Array2::<f64>::zeros((n, 2));
    let mut y = Array1::<f64>::zeros(n);
    for i in 0..n {
        let u0 = lcg01(&mut state);
        let u1 = lcg01(&mut state);
        x[[i, 0]] = u0;
        x[[i, 1]] = u1;
        let score = u0 + u1 - 1.0 + 0.15 * (lcg01(&mut state) - 0.5);
        y[i] = if score > 0.0 { 1.0 } else { 0.0 };
    }
    (x, y)
}

fn print_model(m: &LogisticModel) {
    println!(
        "   intercept = {:.4}, β = [{:.4}, {:.4}]",
        m.intercept, m.coefficients[0], m.coefficients[1]
    );
    println!(
        "   loss = {:.6}, iterations = {}, converged = {}",
        m.loss, m.iterations, m.converged
    );
}

fn rows_at(x: &Array2<f64>, idx: &[usize]) -> Array2<f64> {
    let mut out = Array2::<f64>::zeros((idx.len(), x.ncols()));
    for (r, &i) in idx.iter().enumerate() {
        out.row_mut(r).assign(&x.row(i));
    }
    out
}

fn array1_at(y: &Array1<f64>, idx: &[usize]) -> Array1<f64> {
    Array1::from(take_indices_cloned(&y.to_vec(), idx))
}

fn lcg01(state: &mut u64) -> f64 {
    *state = state.wrapping_mul(1664525).wrapping_add(1013904223);
    (*state as f64) / (u64::MAX as f64)
}
