// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Predictive analytics demo: regression family, k-means, and predicted-vs-expected scoring.
//!
//! Highlights recent Phase A/C work in `symworx-stats`:
//! - OLS / Ridge / Lasso / Elastic Net (`LinearModel`)
//! - k-means clustering
//! - PCA (SVD-backed)
//! - `regression_report` (MAE, RMSE, R², bias, max |e|)
//!
//! Run with:
//! ```bash
//! cargo run -p symworx-stats --example predictive_metrics_demo --features linalg
//! ```

use ndarray::{
    Array1,
    array,
};
use symworx_stats::{
    KMeansConfig,
    elastic_net,
    kmeans,
    lasso,
    ols,
    pca::Pca,
    regression_report,
    ridge,
};

fn main() {
    println!("=== symworx-stats: predictive metrics demo ===\n");

    // --- Synthetic regression data: y = 2x + 1 + small noise ---
    let x = array![[0.0], [1.0], [2.0], [3.0], [4.0], [5.0], [6.0], [7.0]];
    let y = array![1.05, 2.9, 5.1, 7.0, 8.95, 11.1, 12.9, 15.05];

    println!("1) Linear models on y ≈ 2x + 1");
    let ols_m = ols(&x, &y);
    let ridge_m = ridge(&x, &y, 0.5);
    let lasso_m = lasso(&x, &y, 0.05, 500, 1e-8);
    let en_m = elastic_net(&x, &y, 0.05, 0.5, 500, 1e-8);

    println!(
        "   OLS    intercept={:.4}  slope={:.4}",
        ols_m.intercept, ols_m.coefficients[0]
    );
    println!(
        "   Ridge  intercept={:.4}  slope={:.4}  (α=0.5)",
        ridge_m.intercept, ridge_m.coefficients[0]
    );
    println!(
        "   Lasso  intercept={:.4}  slope={:.4}",
        lasso_m.intercept, lasso_m.coefficients[0]
    );
    println!(
        "   ENet   intercept={:.4}  slope={:.4}  (l1_ratio=0.5)",
        en_m.intercept, en_m.coefficients[0]
    );

    // --- Predicted vs expected ---
    println!("\n2) Predicted vs expected (residual e = y − ŷ)");
    let y_hat: Array1<f64> = ols_m.predict(&x);
    let y_slice: Vec<f64> = y.to_vec();
    let yhat_slice: Vec<f64> = y_hat.to_vec();
    let rep = regression_report(&y_slice, &yhat_slice);
    println!("   {rep}");
    println!("   (R² close to 1 and small RMSE ⇒ good fit; bias shows systematic under/over-prediction)");

    // Baseline: always predict the mean of y
    let y_mean = y_slice.iter().sum::<f64>() / y_slice.len() as f64;
    let baseline: Vec<f64> = vec![y_mean; y_slice.len()];
    let base_rep = regression_report(&y_slice, &baseline);
    println!("\n3) Baseline (predict mean ȳ = {y_mean:.3})");
    println!("   {base_rep}");
    println!(
        "   Kuhn-style check: OLS R²={:.4} vs baseline R²={:.4}",
        rep.r2, base_rep.r2
    );

    // --- k-means ---
    println!("\n4) k-means on two Gaussian-like blobs");
    let data = array![[0.0, 0.1], [0.2, -0.1], [0.1, 0.0], [5.0, 5.1], [5.2, 4.9], [4.9, 5.0],];
    let km = kmeans(
        &data,
        &KMeansConfig {
            k: 2,
            seed: 7,
            ..Default::default()
        },
    );
    println!(
        "   labels={:?}  inertia={:.4}  converged={}",
        km.labels, km.inertia, km.converged
    );

    // --- PCA ---
    println!("\n5) PCA (2D → 1 component) on the same blobs");
    let pca = Pca::fit(&data, 1);
    let z = pca.transform(&data);
    println!(
        "   explained variance ratio (fitted comps) = {:?}",
        pca.explained_variance_ratio().to_vec()
    );
    println!("   first scores: {:?}", z.column(0).to_vec());

    println!("\nDone. See also:");
    println!("  cargo run -p symworx-dynamics --example data_driven_dynamics_demo");
    println!("  cargo run -p symworx-signal --example sparse_sensing_demo");
    println!("  cargo run -p symworx-signal --example state_estimation_demo");
}
