// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Classification metrics + standard scaling with binary logistic (simple).
//!
//! ```bash
//! cargo run -p symworx-stats --example classification_metrics_demo
//! ```

use ndarray::array;
use symworx_stats::{
    LogisticConfig,
    StandardScaler,
    classification_report_binary_f64,
    confusion_matrix,
    labels_from_binary_f64,
    logistic_regression,
};

fn main() {
    println!("=== symworx-stats: classification metrics (simple) ===\n");

    // Two features on different scales — scaler matters for GD logistic.
    let x = array![
        [0.0, 100.0],
        [0.1, 110.0],
        [0.2, 90.0],
        [0.3, 105.0],
        [0.4, 95.0],
        [0.6, 200.0],
        [0.7, 210.0],
        [0.8, 190.0],
        [0.9, 205.0],
        [1.0, 195.0],
    ];
    let y = array![0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0];

    println!("1) Standardize features (fit on all rows for this toy demo)");
    let (scaler, x_std) = StandardScaler::fit_transform(&x);
    println!("   mean  = [{:.3}, {:.3}]", scaler.mean[0], scaler.mean[1]);
    println!(
        "   scale = [{:.3}, {:.3}]",
        scaler.scale[0], scaler.scale[1]
    );

    println!("\n2) Fit logistic regression on scaled features");
    let model = logistic_regression(
        &x_std,
        &y,
        &LogisticConfig {
            max_iter: 5000,
            learning_rate: 0.5,
            l2: 0.01,
            tol: 1e-8,
            ..Default::default()
        },
    );
    println!(
        "   intercept={:.4}  β=[{:.4}, {:.4}]  converged={}",
        model.intercept, model.coefficients[0], model.coefficients[1], model.converged
    );

    let yhat = model.predict(&x_std, 0.5);
    let rep = classification_report_binary_f64(&y.to_vec(), &yhat.to_vec());

    println!("\n3) Classification report");
    println!("   {rep}");
    println!(
        "   per-class precision = {:?}",
        rep.precision
            .iter()
            .map(|v| format!("{v:.3}"))
            .collect::<Vec<_>>()
    );
    println!(
        "   per-class recall    = {:?}",
        rep.recall
            .iter()
            .map(|v| format!("{v:.3}"))
            .collect::<Vec<_>>()
    );
    println!(
        "   per-class F1        = {:?}",
        rep.f1.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>()
    );

    let yt = labels_from_binary_f64(&y.to_vec());
    let yp = labels_from_binary_f64(&yhat.to_vec());
    let cm = confusion_matrix(&yt, &yp, Some(2));
    println!("\n4) Confusion matrix (rows=true, cols=pred)");
    println!("         pred0  pred1");
    println!("   true0  {:5}  {:5}", cm[[0, 0]], cm[[0, 1]]);
    println!("   true1  {:5}  {:5}", cm[[1, 0]], cm[[1, 1]]);

    println!("\nDone.");
    println!("  Next roadmap: Gaussian NB, LDA, multiclass logistic, threshold rules.");
}
