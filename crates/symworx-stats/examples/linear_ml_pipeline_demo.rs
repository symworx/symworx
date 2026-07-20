// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! ML-style linear regression: split → fit → hold-out → k-fold CV.
//!
//! For a minimal fit/predict demo (no splits), see `linear_regression_demo`.
//!
//! Requires the `linalg` feature (OLS / Ridge closed form).
//!
//! ```bash
//! cargo run -p symworx-stats --example linear_ml_pipeline_demo --features linalg
//! ```

use ndarray::{
    Array1,
    Array2,
};
use symworx_stats::{
    LinearModel,
    SplitConfig,
    lasso,
    max_train_folds,
    ols,
    regression_report,
    ridge,
    take_indices_cloned,
    train_test_split,
};

fn main() {
    println!("=== symworx-stats: linear regression (ML pipeline) ===\n");

    // ------------------------------------------------------------------
    // 1) Dataset  —  y ≈ 2 x0 − 1.5 x1 + 0.5 + noise
    // ------------------------------------------------------------------
    let (x, y) = make_regression(100, 42);
    let n = x.nrows();
    println!("1) Dataset");
    println!(
        "   n = {n}, n_features = {}  (truth: y = 2 x0 − 1.5 x1 + 0.5 + noise)",
        x.ncols()
    );

    // ------------------------------------------------------------------
    // 2) Index-only split plan
    // ------------------------------------------------------------------
    let test_ratio = 0.3;
    let n_folds = 5;
    println!("\n2) Split plan (indices only)");
    println!(
        "   test_ratio = {test_ratio}, n_train_folds = {n_folds} \
         (max allowed = {})",
        max_train_folds(n, test_ratio)
    );

    let plan = train_test_split(
        n,
        &SplitConfig {
            test_ratio,
            n_train_folds: Some(n_folds),
            shuffle: true,
            seed: 11,
        },
    )
    .expect("valid split for n = 100");

    println!(
        "   outer: train = {}, test = {}  (seed = {:?})",
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

    // ------------------------------------------------------------------
    // 3) Fit family on training set → hold-out test metrics
    // ------------------------------------------------------------------
    println!("\n3) Fit on training set → score hold-out test");

    let ols_m = ols(&x_train, &y_train);
    let ridge_m = ridge(&x_train, &y_train, 0.5);
    let lasso_m = lasso(&x_train, &y_train, 0.05, 2000, 1e-8);

    print_linear("OLS  ", &ols_m);
    print_linear("Ridge", &ridge_m);
    print_linear("Lasso", &lasso_m);

    println!("\n   hold-out test metrics (e = y − ŷ):");
    for (name, model) in [
        ("OLS  ", &ols_m),
        ("Ridge", &ridge_m),
        ("Lasso", &lasso_m),
    ] {
        let yhat = model.predict(&x_test);
        let rep = regression_report(&y_test.to_vec(), &yhat.to_vec());
        println!(
            "   {name}  test  R²={:.4}  RMSE={:.4}  MAE={:.4}  bias={:.4}",
            rep.r2, rep.rmse, rep.mae, rep.bias
        );
    }

    let yhat_tr = ols_m.predict(&x_train);
    let train_rep = regression_report(&y_train.to_vec(), &yhat_tr.to_vec());
    println!(
        "\n   OLS train R²={:.4}  RMSE={:.4}  (compare to test above)",
        train_rep.r2, train_rep.rmse
    );

    // ------------------------------------------------------------------
    // 4) K-fold CV on training indices — pick model by mean val RMSE
    // ------------------------------------------------------------------
    println!("\n4) {n_folds}-fold CV on training set (mean validation RMSE)");

    let mut ols_rmses = Vec::new();
    let mut ridge_rmses = Vec::new();
    let mut lasso_rmses = Vec::new();

    for k in 0..plan.n_folds() {
        let fit_idx = plan.fit_idx(k).unwrap();
        let val_idx = plan.val_idx(k).unwrap();
        let x_fit = rows_at(&x, &fit_idx);
        let y_fit = array1_at(&y, &fit_idx);
        let x_val = rows_at(&x, &val_idx);
        let y_val = array1_at(&y, &val_idx);

        let o = ols(&x_fit, &y_fit);
        let r = ridge(&x_fit, &y_fit, 0.5);
        let l = lasso(&x_fit, &y_fit, 0.05, 2000, 1e-8);

        let o_rmse = rmse_of(&o, &x_val, &y_val);
        let r_rmse = rmse_of(&r, &x_val, &y_val);
        let l_rmse = rmse_of(&l, &x_val, &y_val);
        ols_rmses.push(o_rmse);
        ridge_rmses.push(r_rmse);
        lasso_rmses.push(l_rmse);
        println!(
            "   fold {k}: OLS={o_rmse:.4}  Ridge={r_rmse:.4}  Lasso={l_rmse:.4}"
        );
    }

    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    println!(
        "\n   CV mean RMSE → OLS={:.4}  Ridge={:.4}  Lasso={:.4}",
        mean(&ols_rmses),
        mean(&ridge_rmses),
        mean(&lasso_rmses)
    );

    let (best_name, best_model): (&str, LinearModel) = {
        let m_o = mean(&ols_rmses);
        let m_r = mean(&ridge_rmses);
        let m_l = mean(&lasso_rmses);
        if m_o <= m_r && m_o <= m_l {
            ("OLS", ols_m)
        } else if m_r <= m_l {
            ("Ridge", ridge_m)
        } else {
            ("Lasso", lasso_m)
        }
    };
    let yhat_test = best_model.predict(&x_test);
    let final_rep = regression_report(&y_test.to_vec(), &yhat_test.to_vec());
    println!(
        "   selected by CV: {best_name} → frozen test R²={:.4}  RMSE={:.4}",
        final_rep.r2, final_rep.rmse
    );

    println!("\nDone.");
    println!(
        "  Simple fit/predict: cargo run -p symworx-stats --example linear_regression_demo --features linalg"
    );
}

fn make_regression(n: usize, seed: u64) -> (Array2<f64>, Array1<f64>) {
    let mut state = seed;
    let mut x = Array2::<f64>::zeros((n, 2));
    let mut y = Array1::<f64>::zeros(n);
    for i in 0..n {
        let x0 = lcg01(&mut state) * 2.0 - 0.5;
        let x1 = lcg01(&mut state) * 2.0 - 0.5;
        let noise = (lcg01(&mut state) - 0.5) * 0.4;
        x[[i, 0]] = x0;
        x[[i, 1]] = x1;
        y[i] = 2.0 * x0 - 1.5 * x1 + 0.5 + noise;
    }
    (x, y)
}

fn print_linear(name: &str, m: &LinearModel) {
    println!(
        "   {name}  intercept={:.4}  β=[{:.4}, {:.4}]",
        m.intercept, m.coefficients[0], m.coefficients[1]
    );
}

fn rmse_of(model: &LinearModel, x: &Array2<f64>, y: &Array1<f64>) -> f64 {
    let yhat = model.predict(x);
    regression_report(&y.to_vec(), &yhat.to_vec()).rmse
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
