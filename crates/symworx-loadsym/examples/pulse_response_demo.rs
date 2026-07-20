// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Pulse-response (fitness–fatigue) + multi-day load plan demo.
//!
//! Run with:
//!   cargo run -p symworx-loadsym --example pulse_response_demo

use symworx_loadsym::load::{
    LoadGoal,
    OptimizationThresholds,
    PulseResponseParams,
    estimate_recovery_days,
    forecast_with_constant_load,
    generate_demo_daily_loads,
    optimize_load_plan,
    simulate_pulse_response,
    unit_impulse_response,
};

fn main() {
    println!("=== SymWorx LoadSym Pulse-Response Demo ===\n");

    let params = PulseResponseParams::pmc_defaults();
    println!(
        "PMC params: τ_g={:.0}d  τ_h={:.0}d  k_g={}  k_h={}",
        params.tau_fitness, params.tau_fatigue, params.k_fitness, params.k_fatigue
    );

    // Synthetic history: moderate base + hard week
    let mut loads = generate_demo_daily_loads(28, 70.0, 12.0);
    for l in loads.iter_mut().skip(21) {
        *l = 130.0;
    }
    println!(
        "\nHistory: {} days (last 7 mean ≈ {:.0} TSS)",
        loads.len(),
        loads[loads.len() - 7..].iter().sum::<f64>() / 7.0
    );

    let series = simulate_pulse_response(&loads, &params, None).expect("simulate");
    let s = series.last_state().expect("state");
    println!(
        "End state: CTL(fitness)={:.1}  ATL(fatigue)={:.1}  TSB(form)={:.1}  performance={:.1}",
        s.ctl(),
        s.atl(),
        s.tsb(),
        s.performance
    );

    // Recovery forecast under easy load
    let rest = forecast_with_constant_load(s, 20.0, &params, 7).expect("forecast");
    let after = rest.last_state().unwrap();
    println!(
        "After 7 easy days (20 TSS): form {:+.1} → {:+.1}",
        s.form, after.form
    );

    if let Some(d) = estimate_recovery_days(s, &params, s.form + 10.0, 15.0, 30).unwrap() {
        println!("Days to form +10 under 15 TSS/day: {}", d);
    }

    // Unit impulse (teaching)
    let ir = unit_impulse_response(&params, 14).unwrap();
    println!(
        "\nUnit impulse form (day0..3): {:.3}, {:.3}, {:.3}, {:.3}",
        ir.form[0], ir.form[1], ir.form[2], ir.form[3]
    );

    // Multi-day optimization for each goal
    let thr = OptimizationThresholds {
        horizon_days: 4,
        ..Default::default()
    };
    println!(
        "\n--- Next {}-day plans (default H=4, max 10) ---",
        thr.horizon_days
    );
    for goal in [
        LoadGoal::Recovery,
        LoadGoal::Maintenance,
        LoadGoal::Overload,
    ] {
        match optimize_load_plan(&loads, &params, goal, &thr) {
            Ok(plan) => {
                println!(
                    "\n[{}] success={}  load={:.0}%C  mean={:.0}  C≈{:.0}  form {:.1} → {:.1}",
                    goal.as_str(),
                    plan.success,
                    plan.load_frac * 100.0,
                    plan.mean_plan_load,
                    plan.chronic_ref,
                    plan.form_start,
                    plan.form_end
                );
                println!(
                    "  daily TSS: {}",
                    plan.daily_tss
                        .iter()
                        .map(|v| format!("{:.0}", v))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                for m in &plan.messages {
                    println!("  • {}", m);
                }
            }
            Err(e) => println!("\n[{}] error: {}", goal.as_str(), e),
        }
    }
}
