//! Sparse / compressed sensing demo: OMP, ISTA, DCT basis, Gaussian Φ.
//!
//! Highlights `symworx-signal::processing::sparse_sensing` (Brunton & Kutz style).
//!
//! Run with:
//! ```bash
//! cargo run -p symworx-signal --example sparse_sensing_demo
//! ```

use ndarray::Array1;
use symworx_signal::processing::{
    IstaConfig,
    dct_basis,
    effective_sensing,
    ista,
    measure,
    omp,
    random_gaussian_sensing,
    reconstruct_signal,
};
use symworx_stats::regression_report;

fn main() {
    println!("=== symworx-signal: sparse sensing demo ===\n");

    // Sparse signal in canonical basis
    let n = 32;
    let mut x_true = Array1::zeros(n);
    x_true[3] = 1.5;
    x_true[11] = -2.0;
    x_true[20] = 0.9;

    println!("1) Ground truth: n={n}, sparsity=3 (indices 3, 11, 20)");

    // Underdetermined Gaussian measurements
    let m = 12;
    let phi = random_gaussian_sensing(m, n, 42);
    let y = measure(&phi, &x_true);
    println!("2) Measurements: m={m}  (m ≪ n compressed sensing)");

    // OMP recovery
    let omp_rec = omp(&phi, &y, 3, 1e-10);
    println!(
        "3) OMP  sparsity={}  residual_norm={:.3e}  iterations={}",
        omp_rec.sparsity, omp_rec.residual_norm, omp_rec.iterations
    );
    let true_v: Vec<f64> = x_true.to_vec();
    let omp_v: Vec<f64> = omp_rec.coefficients.to_vec();
    let rep_omp = regression_report(&true_v, &omp_v);
    println!("   predicted vs true coefficients: {rep_omp}");

    // ISTA recovery (identity dictionary — same ambient domain)
    let ista_rec = ista(
        &phi,
        &y,
        &IstaConfig {
            lambda: 0.05,
            max_iter: 400,
            tol: 1e-8,
            step_size: None,
            sparsity_tol: 1e-3,
        },
    );
    println!(
        "\n4) ISTA sparsity={}  residual_norm={:.3e}  iterations={}",
        ista_rec.sparsity, ista_rec.residual_norm, ista_rec.iterations
    );
    let ista_v: Vec<f64> = ista_rec.coefficients.to_vec();
    let rep_ista = regression_report(&true_v, &ista_v);
    println!("   predicted vs true coefficients: {rep_ista}");

    // DCT dictionary sketch
    println!("\n5) DCT basis (orthonormal dictionary for compressible signals)");
    let psi = dct_basis(8);
    let phi_small = random_gaussian_sensing(4, 8, 7);
    let theta = effective_sensing(&phi_small, Some(&psi));
    println!(
        "   Φ is 4×8, Ψ is 8×8 DCT → Θ=ΦΨ is {}×{}",
        theta.nrows(),
        theta.ncols()
    );
    let s = Array1::from(vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    let x_from_s = reconstruct_signal(Some(&psi), &s);
    println!(
        "   reconstruct single DCT mode → ‖x‖₂={:.4}",
        x_from_s.dot(&x_from_s).sqrt()
    );

    println!("\nDone. See also:");
    println!("  cargo run -p symworx-signal --example state_estimation_demo");
    println!("  cargo run -p symworx-stats --example predictive_metrics_demo --features linalg");
}
