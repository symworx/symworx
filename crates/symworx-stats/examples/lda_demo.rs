// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Simple Linear Discriminant Analysis fit / predict.
//!
//! ```bash
//! cargo run -p symworx-stats --example lda_demo --features linalg
//! ```

use ndarray::array;
use symworx_stats::{
    classification_report,
    lda,
};

fn main() {
    println!("=== symworx-stats: LDA (simple) ===\n");

    let x = array![
        [0.0, 0.0],
        [0.2, 0.1],
        [0.1, 0.2],
        [0.0, 0.15],
        [4.0, 4.0],
        [4.2, 3.9],
        [3.9, 4.1],
        [4.1, 4.0],
    ];
    let y = vec![0, 0, 0, 0, 1, 1, 1, 1];

    println!("1) Fit LDA (pooled covariance → linear scores)");
    let model = lda(&x, &y);
    println!("   classes = {:?}", model.classes);
    println!("   priors  = {:?}", model.priors.to_vec());
    println!(
        "   coef rows (per class) = {:?}",
        model.coef.rows().into_iter().map(|r| r.to_vec()).collect::<Vec<_>>()
    );
    println!("   intercepts = {:?}", model.intercept.to_vec());

    let pred = model.predict(&x);
    let rep = classification_report(&y, &pred, Some(2));
    println!("\n2) In-sample report\n   {rep}");

    println!("\n3) Embed note: ship coef + intercept for on-device linear scores");
    println!("   score_k = x · coef_k + intercept_k; predict = argmax_k score_k");

    println!("\nDone.");
}
