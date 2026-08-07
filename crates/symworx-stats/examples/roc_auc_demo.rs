// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Binary ROC curve + AUC (and logistic scores).
//!
//! ```bash
//! cargo run -p symworx-stats --example roc_auc_demo
//! ```

use ndarray::array;
use symworx_stats::{
    LogisticConfig,
    StandardScaler,
    logistic_regression,
    roc_auc,
    roc_curve,
};

fn main() {
    println!("=== symworx-stats: ROC / AUC (simple) ===\n");

    let x = array![[0.0], [0.1], [0.2], [0.3], [0.4], [0.55], [0.65], [0.75], [0.85], [1.0],];
    let y_f = array![0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0];
    let y: Vec<usize> = y_f.iter().map(|&v| v as usize).collect();

    let (scaler, xs) = StandardScaler::fit_transform(&x);
    let _ = scaler;
    let model = logistic_regression(
        &xs,
        &y_f,
        &LogisticConfig {
            max_iter: 5000,
            learning_rate: 0.5,
            ..Default::default()
        },
    );

    // Scores = P(y=1)
    let scores = model.predict_proba(&xs).to_vec();
    let auc = roc_auc(&y, &scores);
    println!("1) Logistic on 1D threshold data");
    println!("   ROC-AUC = {auc:.4}");

    if let Some(curve) = roc_curve(&y, &scores) {
        println!("\n2) ROC curve points (subset)");
        println!("   n_points = {}", curve.fpr.len());
        let step = (curve.fpr.len() / 5).max(1);
        for i in (0..curve.fpr.len()).step_by(step) {
            println!(
                "   FPR={:.3}  TPR={:.3}  thr={:.3}",
                curve.fpr[i], curve.tpr[i], curve.thresholds[i]
            );
        }
        println!("   AUC (from curve) = {:.4}", curve.auc);
    }

    // Perfect ranking sanity
    let y2 = vec![0, 0, 1, 1];
    let s2 = vec![0.1, 0.2, 0.8, 0.9];
    println!("\n3) Perfect scores AUC = {:.1}", roc_auc(&y2, &s2));

    println!("\nDone.");
}
