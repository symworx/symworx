// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! 11v11 planted-label sequence vs the space-action classifier.
//!
//! ```bash
//! cargo run -p symworx-spatialsym --example synthetic_decisions
//! ```

use symworx_spatialsym::{
    classifier_limitations,
    evaluate_space_actions,
    generate_3v3_attack,
    generate_11v11_play,
};

fn main() {
    println!("symworx-spatialsym — synthetic play vs decision classifier\n");

    println!("=== 3v3 attacking-third drill (kept as a short action clip) ===");
    let (b3, f3, _) = generate_3v3_attack();
    println!(
        "  {} agents, {} frames, dt≈{:.2}s",
        b3.num_agents(),
        b3.num_times(),
        b3.times.get(1).copied().unwrap_or(0.1)
    );
    let _ = f3;

    println!("\n=== 11v11 sequence of play (3 min @ 1 Hz) ===");
    let seq = generate_11v11_play();
    println!(
        "  {} agents, {} frames, {} event tags",
        seq.batch.num_agents(),
        seq.batch.num_times(),
        seq.events.len()
    );
    println!("  events:");
    for (f, desc) in &seq.events {
        println!("    t={:>5.0}s  f={f:<3}  {desc}", *f as f64);
    }

    println!("\n--- Classifier A: TUI 1 Hz params (window=2s, radius=15m, look-ahead=2s) ---");
    let decs_a = seq.batch.classify_with_focal_and_params(&seq.focal, 2.0, 15.0, 2.0);
    let eval_a = evaluate_space_actions(&seq.labels, &decs_a);
    for line in eval_a.summary_lines() {
        println!("  {line}");
    }

    println!("\n--- Classifier B: old 3v3 defaults (window=0.5s, radius=10m, look-ahead=0.8s) ---");
    let decs_b = seq.batch.classify_with_focal_and_params(&seq.focal, 0.5, 10.0, 0.8);
    let eval_b = evaluate_space_actions(&seq.labels, &decs_b);
    for line in eval_b.summary_lines() {
        println!("  {line}");
    }

    println!("\n--- Structural limits (not sequence-specific) ---");
    for (i, note) in classifier_limitations().iter().enumerate() {
        println!("  {}. {note}", i + 1);
    }

    // Sample one creation-phase frame: carrier + nearby vs planted.
    let t = 60usize.min(seq.batch.num_times().saturating_sub(1));
    println!("\n--- Frame {t} (creation phase) carrier vs planted ---");
    for i in 0..seq.batch.num_agents() {
        let planted = seq.labels[i][t];
        let pred = decs_a[i][t].action;
        let ball = decs_a[i][t].features.is_ball_carrier;
        if ball || planted != pred && planted != symworx_spatialsym::SpaceAction::Neutral {
            println!("  A{i:<2}  GT={planted:?}  CL={pred:?}  ball={ball}");
        }
    }

    println!("\nDone. Visualize with: cargo run -p symworx-tui --bin symview   then 4, g");
}
