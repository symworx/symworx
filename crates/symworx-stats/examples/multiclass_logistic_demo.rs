// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Multiclass logistic regression via one-vs-rest (simple).
//!
//! ```bash
//! cargo run -p symworx-stats --example multiclass_logistic_demo
//! ```

use ndarray::array;
use symworx_stats::{
    LogisticConfig,
    StandardScaler,
    classification_report,
    logistic_regression_ovr,
    roc_auc_ovr,
};

fn main() {
    println!("=== symworx-stats: multiclass logistic OVR (simple) ===\n");

    // Three 2D blobs
    let x = array![
        [0.0, 0.0],
        [0.15, 0.05],
        [0.05, 0.12],
        [0.2, 0.0],
        [0.1, 0.08],
        [3.5, 3.5],
        [3.6, 3.4],
        [3.4, 3.6],
        [3.55, 3.55],
        [3.45, 3.5],
        [0.0, 3.5],
        [0.1, 3.6],
        [-0.05, 3.4],
        [0.08, 3.55],
        [0.02, 3.48],
    ];
    let y = vec![0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2];

    println!("1) Standardize features, fit OVR logistic (3 classes)");
    let (scaler, xs) = StandardScaler::fit_transform(&x);
    let _ = scaler;

    let model = logistic_regression_ovr(
        &xs,
        &y,
        &LogisticConfig {
            max_iter: 8000,
            learning_rate: 0.4,
            l2: 0.05,
            tol: 1e-8,
            ..Default::default()
        },
    );

    println!("   classes = {:?}", model.classes);
    println!(
        "   mean binary loss = {:.4}, all converged = {}",
        model.mean_loss(),
        model.converged()
    );
    for (c, m) in model.classes.iter().zip(model.models.iter()) {
        println!(
            "   class {c} vs rest: intercept={:.3}  β=[{:.3}, {:.3}]  loss={:.4}",
            m.intercept, m.coefficients[0], m.coefficients[1], m.loss
        );
    }

    let pred = model.predict(&xs);
    let rep = classification_report(&y, &pred, Some(3));
    println!("\n2) In-sample report\n   {rep}");

    let proba = model.predict_proba(&xs);
    let auc = roc_auc_ovr(&y, &proba, Some(&model.classes));
    println!("\n3) Macro OVR ROC-AUC = {auc:.4}");

    println!("\n4) Sample predictions");
    for i in [0, 5, 10] {
        println!(
            "   x≈{:?}  y={}  ŷ={}  P={:?}",
            x.row(i).to_vec(),
            y[i],
            pred[i],
            proba.row(i).iter().map(|p| format!("{p:.3}")).collect::<Vec<_>>()
        );
    }

    println!("\nDone.");
    println!("  Binary only: cargo run -p symworx-stats --example logistic_regression_demo");
}
