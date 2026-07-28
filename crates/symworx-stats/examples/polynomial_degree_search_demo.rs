// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Sweep polynomial degrees and inspect fit quality / sample-size guards.
//!
//! ```bash
//! cargo run -p symworx-stats --example polynomial_degree_search_demo --features linalg
//! ```

use symworx_stats::{
    PolynomialSearchConfig,
    fit_polynomial_degrees,
    fit_polynomial_degrees_with,
};

fn main() {
    println!("=== symworx-stats: polynomial degree search ===\n");

    // Noiseless cubic: y = 1 + 0.5 x − 0.2 x² + 0.05 x³
    let x: Vec<f64> = (0..25).map(|i| (i as f64) * 0.2 - 1.0).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&xi| 1.0 + 0.5 * xi - 0.2 * xi * xi + 0.05 * xi * xi * xi)
        .collect();

    println!("1) n = {}, truth degree = 3, request max_degree = 6", x.len());
    let search = fit_polynomial_degrees(&x, &y, 6).unwrap();
    print!("{search}");
    println!("   best in-sample R² degree = {:?}", search.best_degree_by_r2());

    // Hard stop demo: n = k−1
    println!("\n2) Hard stop: n = 4, request max_degree k = 5 (n = k−1)");
    let x_small = vec![0.0, 1.0, 2.0, 3.0];
    let y_small = vec![1.0, 2.5, 4.0, 7.0];
    let small = fit_polynomial_degrees(&x_small, &y_small, 5).unwrap();
    println!(
        "   fitted max degree = {} (requested 5); n_fits = {}",
        small.max_degree_fitted,
        small.fits.len()
    );
    for w in &small.warnings {
        println!("   · {w}");
    }

    // Soft rule: n < 2 × n_params
    println!("\n3) Soft rule: n = 5, degree 2 (n_params=3, soft min = 6)");
    let x5: Vec<f64> = (0..5).map(|i| i as f64).collect();
    let y5: Vec<f64> = x5.iter().map(|t| t * t).collect();
    let soft = fit_polynomial_degrees(&x5, &y5, 2).unwrap();
    for fit in &soft.fits {
        println!(
            "   degree {}: R²={:.4}  β={:?}",
            fit.degree,
            fit.report.r2,
            fit.coeffs_packed().to_vec()
        );
    }

    // Optional residuals (not returned by default)
    println!("\n4) Optional residuals for degree 1 (return_residuals = true)");
    let with_res = fit_polynomial_degrees_with(
        &x,
        &y,
        &PolynomialSearchConfig {
            max_degree: 1,
            return_residuals: true,
            print_warnings: false,
        },
    )
    .unwrap();
    if let Some(fit) = with_res.fit_for_degree(1) {
        let e = fit.residuals.as_ref().unwrap();
        let show = e.len().min(5);
        println!("   first {show} residuals (y − ŷ): {:?}", &e[..show]);
    }

    println!("\nDone. (Warnings live on the result; print_warnings is opt-in.)");
}
