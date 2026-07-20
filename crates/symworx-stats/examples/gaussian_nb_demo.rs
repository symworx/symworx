// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Simple Gaussian Naive Bayes fit / predict.
//!
//! ```bash
//! cargo run -p symworx-stats --example gaussian_nb_demo
//! ```

use ndarray::array;
use symworx_stats::{
    classification_report,
    gaussian_nb_default,
};

fn main() {
    println!("=== symworx-stats: Gaussian Naive Bayes (simple) ===\n");

    let x = array![
        [0.0, 0.1],
        [0.2, 0.0],
        [0.1, 0.2],
        [0.15, 0.05],
        [5.0, 5.1],
        [5.2, 4.9],
        [4.8, 5.0],
        [5.1, 5.2],
    ];
    let y = vec![0, 0, 0, 0, 1, 1, 1, 1];

    println!("1) Fit Gaussian NB on 2-class blobs");
    let model = gaussian_nb_default(&x, &y);
    println!("   classes = {:?}", model.classes);
    println!(
        "   priors  = {:?}",
        model.log_priors.mapv(f64::exp).to_vec()
    );

    let pred = model.predict(&x);
    let rep = classification_report(&y, &pred, Some(2));
    println!("\n2) In-sample report\n   {rep}");

    let proba = model.predict_proba(&x);
    println!("\n3) Sample probabilities");
    for i in [0, 3, 4, 7] {
        println!(
            "   x={:?}  y={}  P=[{:.3}, {:.3}]  ŷ={}",
            x.row(i).to_vec(),
            y[i],
            proba[[i, 0]],
            proba[[i, 1]],
            pred[i]
        );
    }

    println!("\nDone.");
}
