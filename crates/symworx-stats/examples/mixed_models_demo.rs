// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Linear mixed models: random intercept and linear growth (intercept + slope).
//!
//! Demonstrates simulation, fitting (REML), design helpers, and population vs
//! subject-specific predictions. Requires `linalg` (OpenBLAS-backed Cholesky).
//!
//! ```bash
//! cargo run -p symworx-stats --example mixed_models_demo --features linalg
//! ```

use std::collections::HashMap;

use ndarray::Array2;
use symworx_stats::{
    EstimationMethod,
    LinearGrowthSimSpec,
    LmerConfig,
    RandomInterceptSimSpec,
    RandomTerm,
    center_time,
    generate_linear_growth,
    generate_random_intercept,
    lmer,
    ols,
    regression_report,
    time_powers,
    z_intercept_slope,
};

fn main() {
    println!("=== symworx-stats: linear mixed models ===\n");

    demo_random_intercept();
    println!();
    demo_linear_growth();
    println!();
    demo_design_helpers();

    println!("\nDone.");
    println!("  Run: cargo run -p symworx-stats --example mixed_models_demo --features linalg");
}

/// Multi-subject clustering around a common mean structure (random intercept).
fn demo_random_intercept() {
    println!("────────────────────────────────────────");
    println!("1) Random intercept  y = Xβ + u_g + ε");
    println!("────────────────────────────────────────");

    let spec = RandomInterceptSimSpec {
        n_groups: 40,
        n_per_group: 5,
        intercept: 2.0,
        coefficients: ndarray::array![1.5],
        sigma2: 1.0,
        sigma_u2: 4.0,
        seed: 42,
    };
    let data = generate_random_intercept(&spec).expect("simulate random intercept");
    let n = data.y.len();
    println!(
        "   Simulated: n = {n}, groups = {}, true β0 = {:.2}, β1 = {:.2}",
        spec.n_groups, spec.intercept, spec.coefficients[0]
    );
    println!(
        "              true σ² = {:.2}, σ_u² = {:.2}",
        spec.sigma2, spec.sigma_u2
    );

    let term = RandomTerm::random_intercept("subject", data.groups.clone());
    let fit = lmer(&data.y, &data.x, &[term], &LmerConfig::default()).expect("lmer intercept");

    println!("\n   Fitted (REML):");
    println!(
        "     intercept = {:.4}  (true {:.2})",
        fit.fixed.intercept, spec.intercept
    );
    println!(
        "     slope     = {:.4}  (true {:.2})",
        fit.fixed.coefficients[0], spec.coefficients[0]
    );
    println!("     σ²        = {:.4}  (true {:.2})", fit.sigma2, spec.sigma2);
    println!(
        "     σ_u²      = {:.4}  (true {:.2})",
        fit.sigma_u2("subject").unwrap_or(f64::NAN),
        spec.sigma_u2
    );
    println!(
        "     loglik    = {:.3}  converged = {}  iters = {}",
        fit.loglik, fit.converged, fit.iterations
    );

    // Complete-pooling OLS baseline (ignores subject)
    let ols_m = ols(&data.x, &data.y);
    println!("\n   OLS baseline (no random effects):");
    println!(
        "     intercept = {:.4}  slope = {:.4}",
        ols_m.intercept, ols_m.coefficients[0]
    );

    let y_pop = fit.predict(&data.x);
    let mut gmap: HashMap<String, &[usize]> = HashMap::new();
    gmap.insert("subject".into(), data.groups.as_slice().expect("contiguous groups"));
    let zmap: HashMap<String, &Array2<f64>> = HashMap::new(); // q=1 → ones implied
    let y_cond = fit.predict_conditional(&data.x, &gmap, &zmap).expect("conditional");

    let yv = data.y.to_vec();
    let rep_pop = regression_report(&yv, &y_pop.to_vec());
    let rep_cond = regression_report(&yv, &y_cond.to_vec());
    println!("\n   In-sample fit quality:");
    println!(
        "     population  (RE=0):  R² = {:.4}  RMSE = {:.4}",
        rep_pop.r2, rep_pop.rmse
    );
    println!(
        "     conditional (BLUP):  R² = {:.4}  RMSE = {:.4}",
        rep_cond.r2, rep_cond.rmse
    );

    // A few subject BLUPs
    if let Some(u) = fit.ranef("subject") {
        println!("\n   BLUPs û (first 5 subjects):");
        for g in 0..5.min(u.nrows()) {
            println!("     subject {g}: û = {:+.4}", u[[g, 0]]);
        }
    }
}

/// Longitudinal trajectories with subject-specific intercept and slope.
fn demo_linear_growth() {
    println!("────────────────────────────────────────");
    println!("2) Linear growth  y = β0 + β1 t + u0_g + u1_g·t + ε");
    println!("────────────────────────────────────────");

    let mut g_true = Array2::<f64>::zeros((2, 2));
    g_true[[0, 0]] = 4.0; // intercept variance
    g_true[[1, 1]] = 0.25; // slope variance
    g_true[[0, 1]] = 0.4;
    g_true[[1, 0]] = 0.4;

    let spec = LinearGrowthSimSpec {
        n_groups: 50,
        n_per_group: 6,
        n_per: None,
        intercept: 1.0,
        slope: 0.5,
        sigma2: 0.5,
        re_cov: g_true.clone(),
        seed: 11,
    };
    let data = generate_linear_growth(&spec).expect("simulate growth");
    println!(
        "   Simulated: n = {}, subjects = {}, times = 0..{}",
        data.y.len(),
        spec.n_groups,
        spec.n_per_group - 1
    );
    println!(
        "              true β0 = {:.2}, β1 = {:.2}, σ² = {:.2}",
        spec.intercept, spec.slope, spec.sigma2
    );
    println!(
        "              true G = [[ {:.2}, {:.2} ], [ {:.2}, {:.2} ]]",
        g_true[[0, 0]],
        g_true[[0, 1]],
        g_true[[1, 0]],
        g_true[[1, 1]]
    );

    let term = RandomTerm::linear_growth("id", data.groups.clone(), &data.time).expect("term");
    let z = term.z_cols.clone().expect("Z = [1, t]");

    let cfg = LmerConfig {
        method: EstimationMethod::Reml,
        max_iter: 400,
        n_restarts: 4,
        learning_rate: 0.08,
        line_search: true,
        ..LmerConfig::default()
    };
    let fit = lmer(&data.y, &data.x, &[term], &cfg).expect("lmer growth");

    let gest = fit.re_covariance("id").expect("G");
    println!("\n   Fitted (REML, unstructured G):");
    println!(
        "     intercept = {:.4}  (true {:.2})",
        fit.fixed.intercept, spec.intercept
    );
    println!(
        "     slope     = {:.4}  (true {:.2})",
        fit.fixed.coefficients[0], spec.slope
    );
    println!("     σ²        = {:.4}  (true {:.2})", fit.sigma2, spec.sigma2);
    println!("     G         = [[ {:.4}, {:.4} ],", gest[[0, 0]], gest[[0, 1]]);
    println!("                  [ {:.4}, {:.4} ]]", gest[[1, 0]], gest[[1, 1]]);
    println!(
        "     loglik    = {:.3}  converged = {}  iters = {}",
        fit.loglik, fit.converged, fit.iterations
    );

    let y_pop = fit.predict(&data.x);
    let mut gmap: HashMap<String, &[usize]> = HashMap::new();
    gmap.insert("id".into(), data.groups.as_slice().expect("groups"));
    let mut zmap: HashMap<String, &Array2<f64>> = HashMap::new();
    zmap.insert("id".into(), &z);
    let y_cond = fit.predict_conditional(&data.x, &gmap, &zmap).expect("conditional");

    let yv = data.y.to_vec();
    let rep_pop = regression_report(&yv, &y_pop.to_vec());
    let rep_cond = regression_report(&yv, &y_cond.to_vec());
    println!("\n   In-sample fit quality:");
    println!(
        "     population  (RE=0):  R² = {:.4}  RMSE = {:.4}",
        rep_pop.r2, rep_pop.rmse
    );
    println!(
        "     conditional (BLUP):  R² = {:.4}  RMSE = {:.4}",
        rep_cond.r2, rep_cond.rmse
    );

    // Subject-specific trajectory snapshot (one subject)
    if let Some(u) = fit.ranef("id") {
        let sid = 0usize;
        println!("\n   Subject {sid} trajectory (time, y, ŷ_pop, ŷ_cond):");
        println!("     BLUPs: û0 = {:+.4}, û1 = {:+.4}", u[[sid, 0]], u[[sid, 1]]);
        for i in 0..data.y.len() {
            if data.groups[i] != sid {
                continue;
            }
            println!(
                "     t={:.0}  y={:6.3}  ŷ_pop={:6.3}  ŷ_cond={:6.3}",
                data.time[i], data.y[i], y_pop[i], y_cond[i]
            );
        }
    }

    println!("\n   Model summary:\n{}", indent(&fit.summary(), "   "));
}

/// Time centering and polynomial fixed design with a random intercept.
fn demo_design_helpers() {
    println!("────────────────────────────────────────");
    println!("3) Design helpers (center time, powers, Z)");
    println!("────────────────────────────────────────");

    let data = generate_linear_growth(&LinearGrowthSimSpec {
        n_groups: 30,
        n_per_group: 5,
        seed: 7,
        ..Default::default()
    })
    .expect("sim");

    let (t_c, mean_t) = center_time(&data.time).expect("center");
    println!("   Mean time = {mean_t:.3}; centered time has mean ≈ 0");

    // Fixed: centered time only (intercept via fit_intercept)
    let x = time_powers(&t_c, 1);
    let z = z_intercept_slope(&t_c);
    let term = RandomTerm {
        name: "id".into(),
        groups: data.groups.clone(),
        z_cols: Some(z.clone()),
        cov_structure: symworx_stats::CovStructure::Unstructured,
    };

    let fit = lmer(
        &data.y,
        &x,
        &[term],
        &LmerConfig {
            n_restarts: 3,
            max_iter: 300,
            ..LmerConfig::default()
        },
    )
    .expect("lmer centered");

    println!(
        "   Fit with centered time: β0 = {:.4}, β_t = {:.4}, σ² = {:.4}",
        fit.fixed.intercept, fit.fixed.coefficients[0], fit.sigma2
    );
    println!("   Z shape = {:?}, re_dim = {}", z.shape(), fit.re_dim["id"]);
    println!("   (Centering often improves interpretability of the intercept.)");
}

fn indent(s: &str, prefix: &str) -> String {
    s.lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}
