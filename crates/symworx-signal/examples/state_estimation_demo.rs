//! State estimation demo: linear Kalman helpers, EKF, and UKF.
//!
//! Highlights recent nonlinear filter work in `symworx-signal`.
//!
//! Run with:
//! ```bash
//! cargo run -p symworx-signal --example state_estimation_demo
//! ```

use ndarray::{
    Array1,
    Array2,
    array,
};
use symworx_signal::filters::{
    ExtendedKalmanFilter,
    KalmanFilter,
    UkfParams,
    UnscentedKalmanFilter,
};
use symworx_stats::regression_report;

fn main() {
    println!("=== symworx-signal: state estimation demo ===\n");

    // --- Linear Kalman: constant-velocity tracker ---
    println!("1) KalmanFilter::constant_velocity_1d (noisy ramp)");
    let mut kf = KalmanFilter::constant_velocity_1d(1.0, 1e-4, 0.25);
    let mut true_pos = Vec::new();
    let mut est_pos = Vec::new();
    for t in 0..25 {
        let z = t as f64 + 0.15 * ((t as f64) * 0.7).sin(); // noisy observation of position ≈ t
        kf.predict(None);
        kf.update(&array![z]);
        true_pos.push(t as f64);
        est_pos.push(kf.state()[0]);
    }
    let rep_cv = regression_report(&true_pos, &est_pos);
    println!("   filtered position vs true ramp: {rep_cv}");
    println!(
        "   final state [pos, vel] = [{:.3}, {:.3}]",
        kf.state()[0],
        kf.state()[1]
    );

    // --- Random walk smoother-style tracking of a level ---
    println!("\n2) KalmanFilter::random_walk (2 independent channels)");
    let mut rw = KalmanFilter::random_walk(2, 0.05, 0.2);
    for k in 0..10 {
        let z = array![k as f64 * 0.1, -(k as f64) * 0.05];
        rw.predict(None);
        rw.update(&z);
    }
    println!("   state after 10 steps: {:?}", rw.state().to_vec());

    // --- EKF: estimate θ from sin(θ) measurements ---
    println!("\n3) ExtendedKalmanFilter — measure sin(θ), recover θ");
    let true_theta = 0.55_f64;
    let mut ekf =
        ExtendedKalmanFilter::new(array![0.0], array![[1.0]], array![[1e-5]], array![[0.02]]);
    let f = |x: &Array1<f64>, _: Option<&Array1<f64>>| array![x[0]]; // static state
    let mut ekf_hist = Vec::new();
    let mut truth = Vec::new();
    for _ in 0..40 {
        ekf.predict_fd(&f, None, 1e-6);
        ekf.update_fd(&array![true_theta.sin()], |x| array![x[0].sin()], 1e-6);
        ekf_hist.push(ekf.state()[0]);
        truth.push(true_theta);
    }
    let rep_ekf = regression_report(&truth, &ekf_hist);
    println!("   EKF θ trajectory vs constant truth: {rep_ekf}");
    println!("   final θ̂ = {:.4}  (true = {true_theta})", ekf.state()[0]);

    // --- UKF: same nonlinear measurement ---
    println!("\n4) UnscentedKalmanFilter — same sin(θ) problem");
    let mut ukf = UnscentedKalmanFilter::with_params(
        array![0.0],
        array![[1.0]],
        array![[1e-5]],
        array![[0.02]],
        UkfParams {
            alpha: 0.1,
            beta: 2.0,
            kappa: 0.0,
        },
    );
    let h = |x: &Array1<f64>| array![x[0].sin()];
    let mut ukf_hist = Vec::new();
    for _ in 0..40 {
        ukf.predict(&f, None);
        ukf.update(&array![true_theta.sin()], &h);
        ukf_hist.push(ukf.state()[0]);
    }
    let rep_ukf = regression_report(&truth, &ukf_hist);
    println!("   UKF θ trajectory vs constant truth: {rep_ukf}");
    println!("   final θ̂ = {:.4}  (true = {true_theta})", ukf.state()[0]);

    // --- Controlled discrete LTI + Kalman ---
    println!("\n5) from_discrete_lti with control input B");
    let f_mat = Array2::eye(2);
    let h_mat = array![[1.0, 0.0]];
    let q = Array2::eye(2) * 0.01;
    let r = array![[0.1]];
    let b = array![[1.0], [0.0]];
    let mut controlled = KalmanFilter::from_discrete_lti(
        f_mat,
        h_mat,
        q,
        r,
        array![0.0, 0.0],
        Array2::eye(2) * 10.0,
        Some(b),
    );
    controlled.predict(Some(&array![0.75]));
    println!("   after u=[0.75]: state={:?}", controlled.state().to_vec());

    println!("\nDone. See also:");
    println!("  cargo run -p symworx-signal --example sparse_sensing_demo");
    println!("  cargo run -p symworx-signal --example windowed_rr_features");
}
