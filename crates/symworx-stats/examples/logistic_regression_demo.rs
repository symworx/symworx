// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Simple binary logistic regression: fit and predict (no split / CV).
//!
//! For a full ML pipeline (train/test + k-fold), see
//! `logistic_ml_pipeline_demo`.
//!
//! ```bash
//! cargo run -p symworx-stats --example logistic_regression_demo
//! ```

use ndarray::array;
use symworx_stats::{
    LogisticConfig,
    logistic_regression,
};

fn main() {
    println!("=== symworx-stats: logistic regression (simple) ===\n");

    // y = 1 when x is large
    let x = array![
        [0.0],
        [0.1],
        [0.2],
        [0.3],
        [0.4],
        [0.6],
        [0.7],
        [0.8],
        [0.9],
        [1.0],
    ];
    let y = array![0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0];

    println!(
        "1) Data: n = {}, n_features = 1 (threshold around x = 0.5)",
        x.nrows()
    );

    let model = logistic_regression(
        &x,
        &y,
        &LogisticConfig {
            max_iter: 5000,
            learning_rate: 0.5,
            l2: 0.0,
            tol: 1e-8,
            ..Default::default()
        },
    );

    println!("\n2) Fitted model");
    println!(
        "   intercept = {:.4}, coefficient = {:.4}",
        model.intercept, model.coefficients[0]
    );
    println!(
        "   loss = {:.6}, iterations = {}, converged = {}",
        model.loss, model.iterations, model.converged
    );

    println!("\n3) Predictions on the training points");
    let proba = model.predict_proba(&x);
    let yhat = model.predict(&x, 0.5);
    for i in 0..x.nrows() {
        println!(
            "   x={:.1}  y={:.0}  p(y=1)={:.3}  ŷ={:.0}",
            x[[i, 0]],
            y[i],
            proba[i],
            yhat[i]
        );
    }

    let acc = model.accuracy(&x, &y, 0.5);
    println!("\n4) Training accuracy = {acc:.3}");

    println!("\nDone.");
    println!(
        "  ML pipeline version: cargo run -p symworx-stats --example logistic_ml_pipeline_demo"
    );
}
