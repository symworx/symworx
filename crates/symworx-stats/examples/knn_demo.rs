// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Multiclass k-NN classifier (simple fit / predict).
//!
//! Stores the training set — workstation/teaching use; not an MCU default.
//!
//! ```bash
//! cargo run -p symworx-stats --example knn_demo
//! ```

use ndarray::array;
use symworx_stats::{
    KnnClassifier,
    KnnConfig,
    KnnMetric,
    classification_report,
    roc_auc_ovr,
};

fn main() {
    println!("=== symworx-stats: k-NN multiclass (simple) ===\n");

    // Three clusters
    let x = array![
        [0.0, 0.0],
        [0.1, 0.05],
        [0.05, 0.1],
        [0.15, 0.0],
        [3.0, 0.0],
        [3.1, 0.1],
        [2.9, -0.05],
        [3.05, 0.05],
        [0.0, 3.0],
        [0.1, 3.1],
        [-0.05, 2.9],
        [0.05, 3.05],
    ];
    let y = vec![0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2];

    println!("1) Fit k-NN (k=3, Euclidean)");
    let clf = KnnClassifier::fit(
        &x,
        &y,
        KnnConfig {
            k: 3,
            metric: KnnMetric::Euclidean,
            weighted: false,
        },
    );
    println!(
        "   n_train={}, n_features={}, classes={:?}",
        clf.n_train(),
        clf.n_features(),
        clf.classes
    );

    let pred = clf.predict(&x);
    let rep = classification_report(&y, &pred, Some(3));
    println!("\n2) In-sample classification report\n   {rep}");

    let proba = clf.predict_proba(&x);
    let auc = roc_auc_ovr(&y, &proba, Some(&clf.classes));
    println!("\n3) Macro one-vs-rest ROC-AUC (from vote fractions) = {auc:.4}");

    println!("\n4) Query points");
    let q = array![[0.02, 0.02], [3.0, 0.02], [0.0, 3.0]];
    let qp = clf.predict(&q);
    let qproba = clf.predict_proba(&q);
    for i in 0..q.nrows() {
        println!(
            "   x={:?} → ŷ={}  P={:?}",
            q.row(i).to_vec(),
            qp[i],
            qproba.row(i).to_vec()
        );
    }

    println!("\nDone.");
}
