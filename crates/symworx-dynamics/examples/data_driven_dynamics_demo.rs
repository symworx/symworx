//! Data-driven dynamics demo: DMD, EDMD, SINDy, SINDYc, and LTI + PID.
//!
//! Highlights recent Phase B work in `symworx-dynamics`, scored with
//! `symworx_stats::regression_report` where useful.
//!
//! Run with:
//! ```bash
//! cargo run -p symworx-dynamics --example data_driven_dynamics_demo
//! ```

use ndarray::{
    Array1,
    Array2,
    array,
};
use symworx_dynamics::{
    Dictionary,
    DmdConfig,
    EdmdConfig,
    LtiDiscrete,
    Pid,
    PidConfig,
    SindyConfig,
    SindycConfig,
    dmd,
    edmd,
    sindy,
    sindyc,
};
use symworx_stats::regression_report;

fn simulate_linear(a: &Array2<f64>, x0: Array1<f64>, steps: usize) -> Array2<f64> {
    let n = x0.len();
    let mut snaps = Array2::zeros((n, steps));
    let mut x = x0;
    for k in 0..steps {
        snaps.column_mut(k).assign(&x);
        x = a.dot(&x);
    }
    snaps
}

fn main() {
    println!("=== symworx-dynamics: data-driven dynamics demo ===\n");

    // Discrete linear system with known eigenvalues 0.9 ± 0.2i
    let a = array![[0.9, -0.2], [0.2, 0.9]];
    let snaps = simulate_linear(&a, array![1.0, 0.0], 40);

    // --- DMD ---
    println!("1) DMD on linear oscillator snapshots");
    let dmd_model = dmd(
        &snaps,
        &DmdConfig {
            rank: Some(2),
            dt: Some(1.0),
            ..Default::default()
        },
    );
    println!("   rank = {}", dmd_model.rank);
    print!("   eigenvalues:");
    for lam in dmd_model.eigenvalues.iter() {
        print!("  {:.4}+{:.4}i", lam.re, lam.im);
    }
    println!();

    // Score reconstruction of first state component over time
    let mut y = Vec::with_capacity(snaps.ncols());
    let mut yhat = Vec::with_capacity(snaps.ncols());
    for k in 0..snaps.ncols() {
        y.push(snaps[[0, k]]);
        yhat.push(dmd_model.predict_discrete(k)[0]);
    }
    let rep = regression_report(&y, &yhat);
    println!("   predicted vs actual (x₀ component): {rep}");

    // --- EDMD ---
    println!("\n2) EDMD (identity dictionary ≡ full-state linear model)");
    let edmd_model = edmd(
        &snaps,
        &EdmdConfig {
            dictionary: Dictionary::Identity,
            ridge: 0.0,
        },
    );
    println!(
        "   K ≈ A  (rel. fit error = {:.2e})",
        edmd_model.relative_fit_error
    );
    let x = array![1.0, 0.0];
    let pred = edmd_model.predict_one(&x);
    let true_next = a.dot(&x);
    println!(
        "   one-step: pred={:?}  true={:?}",
        pred.to_vec(),
        true_next.to_vec()
    );

    // --- SINDy (continuous-style via small Euler plant) ---
    println!("\n3) SINDy on ẋ = A_c x  (Euler-integrated training data)");
    let a_c = array![[-0.5, 0.1], [0.0, -0.3]];
    let dt = 0.02;
    let mut cont = Array2::zeros((2, 200));
    let mut xc = array![1.0, 0.5];
    for k in 0..200 {
        cont.column_mut(k).assign(&xc);
        xc = &xc + &(&a_c.dot(&xc) * dt);
    }
    let sindy_model = sindy(
        &cont,
        dt,
        &SindyConfig {
            dictionary: Dictionary::Polynomial {
                max_degree: 2,
                include_constant: true,
            },
            threshold: 0.05,
            max_iter: 15,
            ridge: 1e-10,
        },
    );
    println!(
        "   library_dim={}  sparsity={}  fit_err={:.3e}",
        sindy_model.library_dim,
        sindy_model.sparsity(1e-6),
        sindy_model.relative_fit_error
    );
    let f = sindy_model.rhs(&array![1.0, 0.5]);
    let f_true = a_c.dot(&array![1.0, 0.5]);
    println!(
        "   rhs at [1,0.5]: model={:?}  true={:?}",
        f.to_vec(),
        f_true.to_vec()
    );

    // --- SINDYc ---
    println!("\n4) SINDYc on ẋ = -0.5 x + u  (scalar, multi-sine input)");
    let t = 400;
    let mut u_mat = Array2::zeros((1, t));
    let mut x_mat = Array2::zeros((1, t));
    let mut xs = 0.2_f64;
    for k in 0..t {
        let tk = k as f64 * dt;
        let u = (2.0 * std::f64::consts::PI * 0.5 * tk).sin();
        u_mat[[0, k]] = u;
        x_mat[[0, k]] = xs;
        xs += dt * (-0.5 * xs + u);
    }
    let sindyc_model = sindyc(
        &x_mat,
        &u_mat,
        dt,
        &SindycConfig {
            state_dictionary: Dictionary::Polynomial {
                max_degree: 2,
                include_constant: true,
            },
            control_dictionary: Dictionary::Identity,
            include_products: false,
            threshold: 0.05,
            max_iter: 15,
            ridge: 1e-10,
        },
    );
    println!(
        "   Ξ (library × state) =\n{}",
        format_matrix(&sindyc_model.xi)
    );
    let f_c = sindyc_model.rhs(&array![0.5], &array![1.0]);
    println!(
        "   f(0.5, u=1) model={:.4}  true={:.4}",
        f_c[0],
        -0.5 * 0.5 + 1.0
    );

    // --- LTI + PID ---
    println!("\n5) LTI scalar plant + PID regulation");
    let plant = LtiDiscrete::scalar(0.95, 0.1);
    let mut pid = Pid::new(PidConfig {
        kp: 2.0,
        ki: 0.4,
        kd: 0.05,
        dt: 1.0,
        integral_limit: Some(10.0),
        output_limit: Some(20.0),
    });
    let setpoint = 1.0;
    let mut x_pid = array![0.0];
    for _ in 0..60 {
        let e = setpoint - x_pid[0];
        let u = pid.step(e);
        let (xn, _) = plant.step(&x_pid, Some(&array![u]));
        x_pid = xn;
    }
    println!(
        "   after 60 steps: x={:.4}  (setpoint={setpoint})",
        x_pid[0]
    );

    println!("\nDone. See also:");
    println!("  cargo run -p symworx-stats --example predictive_metrics_demo --features linalg");
    println!("  cargo run -p symworx-signal --example state_estimation_demo");
}

fn format_matrix(a: &Array2<f64>) -> String {
    let mut s = String::new();
    for i in 0..a.nrows() {
        s.push_str("     [");
        for j in 0..a.ncols() {
            if j > 0 {
                s.push_str(", ");
            }
            s.push_str(&format!("{:.4}", a[[i, j]]));
        }
        s.push_str("]\n");
    }
    s
}
