// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Simple linear regression: OLS / Ridge / Lasso fit and predict (no split / CV).
//!
//! For a full ML pipeline (train/test + k-fold), see
//! `linear_ml_pipeline_demo`.
//!
//! Requires the `linalg` feature (OLS / Ridge).
//!
//! ```bash
//! cargo run -p symworx-stats --example linear_regression_demo --features linalg
//! ```

use ndarray::array;
use symworx_stats::{
    elastic_net,
    lasso,
    ols,
    regression_report,
    ridge,
};

fn main() {
    println!("=== symworx-stats: linear regression (simple) ===\n");

    // y ≈ 2x + 1 + small noise
    let x = array![[0.0], [1.0], [2.0], [3.0], [4.0], [5.0], [6.0], [7.0]];
    let y = array![1.05, 2.9, 5.1, 7.0, 8.95, 11.1, 12.9, 15.05];

    println!("1) Data: n = {}, y ≈ 2x + 1", x.nrows());

    println!("\n2) Fit models on all rows");
    let ols_m = ols(&x, &y);
    let ridge_m = ridge(&x, &y, 0.5);
    let lasso_m = lasso(&x, &y, 0.05, 500, 1e-8);
    let en_m = elastic_net(&x, &y, 0.05, 0.5, 500, 1e-8);

    println!(
        "   OLS     intercept={:.4}  slope={:.4}",
        ols_m.intercept, ols_m.coefficients[0]
    );
    println!(
        "   Ridge   intercept={:.4}  slope={:.4}  (α=0.5)",
        ridge_m.intercept, ridge_m.coefficients[0]
    );
    println!(
        "   Lasso   intercept={:.4}  slope={:.4}",
        lasso_m.intercept, lasso_m.coefficients[0]
    );
    println!(
        "   ENet    intercept={:.4}  slope={:.4}  (l1_ratio=0.5)",
        en_m.intercept, en_m.coefficients[0]
    );

    println!("\n3) OLS predicted vs expected (residual e = y − ŷ)");
    let yhat = ols_m.predict(&x);
    let rep = regression_report(&y.to_vec(), &yhat.to_vec());
    println!("   {rep}");

    println!("\n4) Pointwise OLS predictions");
    for i in 0..x.nrows() {
        println!(
            "   x={:.0}  y={:.2}  ŷ={:.2}  e={:.3}",
            x[[i, 0]],
            y[i],
            yhat[i],
            y[i] - yhat[i]
        );
    }

    println!("\nDone.");
    println!("  ML pipeline version: cargo run -p symworx-stats --example linear_ml_pipeline_demo --features linalg");
}
