// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Path linearity and pairwise effort / directional phase on synthetic drills.
//!
//! ```bash
//! cargo run -p symworx-spatialsym --example path_and_phase
//! ```

use symworx_spatialsym::{
    AgentTrajectories,
    PhaseWindow,
    Point2,
    Vec2,
    generate_3v3_attack,
    generate_curved_trajectory,
    generate_linear_trajectory,
    pairwise_closing,
    pairwise_directional_phase,
    pairwise_effort_phase,
    pairwise_effort_phase_series,
    path_linearity,
};

fn from_speeds(start: Point2, dir: Vec2, speeds: &[f64]) -> (Vec<f64>, Vec<Point2>) {
    let unit = dir.normalize();
    let mut times = vec![0.0];
    let mut pos = vec![start];
    let mut p = start;
    for (i, &s) in speeds.iter().enumerate() {
        p = p + unit * s;
        times.push((i + 1) as f64);
        pos.push(p);
    }
    (times, pos)
}

fn print_pair(label: &str, a: &[Point2], b: &[Point2], times: &[f64], cfg: &PhaseWindow) {
    println!("\n{label}");
    let effort = pairwise_effort_phase(a, b, times, cfg).unwrap();
    let dir = pairwise_directional_phase(a, b, times, cfg).unwrap();
    match effort {
        Some(e) => println!(
            "  effort  in={}  out={}  frac={:?}  sign={:?}  lag={:.2}s",
            e.in_phase_events, e.out_of_phase_events, e.event_in_phase_fraction, e.sign_agree_fraction, e.lag_sec
        ),
        None => println!("  effort  (none)"),
    }
    match dir {
        Some(d) => println!(
            "  dir     dominant={:?}  counts={:?}  heading_undef={}",
            d.dominant.map(|r| r.short_label()),
            d.counts,
            d.heading_undefined
        ),
        None => println!("  dir     (none)"),
    }
}

fn main() {
    println!("symworx-spatialsym — path linearity + pairwise phase\n");

    let start = Point2::origin();
    let vel = Vec2::new(2.0, 0.0);
    let straight = generate_linear_trajectory(start, vel, 4.0, 0.05);
    let weave = generate_curved_trajectory(start, vel, 4.0, 0.05, 1.5, 3.0);
    let s = path_linearity(&straight).unwrap();
    let w = path_linearity(&weave).unwrap();
    println!("Path linearity (same duration / base speed):");
    println!(
        "  straight  path={:.2} m  net={:.2} m  eff={:.3}  rms={:.3} m",
        s.path_length_m,
        s.net_displacement_m,
        s.efficiency,
        s.rms_dev_m.unwrap_or(f64::NAN)
    );
    println!(
        "  weave     path={:.2} m  net={:.2} m  eff={:.3}  rms={:.3} m",
        w.path_length_m,
        w.net_displacement_m,
        w.efficiency,
        w.rms_dev_m.unwrap_or(f64::NAN)
    );

    let wins = symworx_spatialsym::path_linearity_windows(&weave, 5.0);
    if let Some(first) = wins.first() {
        println!(
            "  weave 5 m windows: {} slices, first path={:.2} m eff={:.3}",
            wins.len(),
            first.path_length_m,
            first.efficiency
        );
    }

    let cfg = PhaseWindow {
        accel_threshold: 0.4,
        ..PhaseWindow::default()
    };

    let together = [1.0, 2.0, 1.0];
    let (times, a) = from_speeds(Point2::origin(), Vec2::new(1.0, 0.0), &together);
    let (_, b) = from_speeds(Point2::new(0.0, 2.0), Vec2::new(1.0, 0.0), &together);
    print_pair("Together (same effort, same heading)", &a, &b, &times, &cfg);

    let speed_up = [1.0, 2.5, 2.5];
    let (times, a) = from_speeds(Point2::origin(), Vec2::new(1.0, 0.0), &speed_up);
    let (_, b) = from_speeds(Point2::new(0.0, 2.0), Vec2::new(-1.0, 0.0), &speed_up);
    print_pair(
        "Spatially opposed (same effort, opposite heading)",
        &a,
        &b,
        &times,
        &cfg,
    );

    let (times, a) = from_speeds(Point2::origin(), Vec2::new(1.0, 0.0), &[1.0, 2.5]);
    let (_, b) = from_speeds(Point2::new(0.0, 2.0), Vec2::new(1.0, 0.0), &[2.5, 1.0]);
    print_pair("One-go-one-brake (opposite effort, same heading)", &a, &b, &times, &cfg);

    let batch = AgentTrajectories::new(times.clone(), vec![a, b])
        .unwrap()
        .with_groups(vec![0, 0])
        .unwrap();
    let groups = batch.per_group_summaries(0.4, 1.0, None);
    if let Some(g) = groups.first() {
        println!(
            "\nGroup 0 roll-up  effort-in={:?}  dir-in={:?}",
            g.mean_effort_in_phase, g.mean_directional_in_phase
        );
    }

    let mut win_cfg = cfg.clone();
    win_cfg.window_sec = 1.0;
    let (times, a) = from_speeds(Point2::origin(), Vec2::new(1.0, 0.0), &[1.0, 2.0, 2.0, 3.0, 3.5]);
    let (_, b) = from_speeds(Point2::new(0.0, 2.0), Vec2::new(1.0, 0.0), &[1.0, 2.0, 2.0, 1.0, 0.4]);
    let series = pairwise_effort_phase_series(&a, &b, &times, &win_cfg).unwrap();
    println!("\nTimeline (together then B brakes), window=1s:");
    if let (Some(first), Some(last)) = (series.first(), series.last()) {
        println!(
            "  early frac={:?}  late frac={:?}",
            first.as_ref().and_then(|s| s.event_in_phase_fraction),
            last.as_ref().and_then(|s| s.event_in_phase_fraction)
        );
    }

    let times = vec![0.0, 1.0, 2.0, 3.0];
    let runner = vec![
        Point2::new(0.0, 0.0),
        Point2::new(1.0, 0.0),
        Point2::new(3.0, 0.0),
        Point2::new(6.0, 0.0),
    ];
    let stand = vec![Point2::new(10.0, 0.0); 4];
    let close = pairwise_closing(&runner, &stand, &times, &cfg).unwrap();
    println!(
        "\nClosing (A speeds up at standing B): mean a_close i→j = {:?}",
        close.as_ref().and_then(|c| c.mean_i_toward_j)
    );

    let (batch3, focal3, _) = generate_3v3_attack();
    let dims = batch3.playing_dimensions.expect("3v3 pitch");
    let marks = batch3.play_area_markings.expect("IFAB markings");
    println!(
        "\n3v3 attack (+x / up on plan), FIFA {:.0}×{:.0} m + IFAB markings:",
        dims.length_m, dims.width_m
    );
    println!(
        "  outer box {:.1}×{:.1} m  inner {:.1}×{:.1} m  goal {:.2} m  PK {:.0} m  circle r={:.2} m",
        marks.outer_end.depth_m,
        marks.outer_end.width_m,
        marks.inner_end.depth_m,
        marks.inner_end.width_m,
        marks.goal.width_m,
        marks.penalty_mark.from_goal_line_m,
        marks.center_circle.radius_m
    );
    let players = batch3.per_player_summaries(0.8, 1.0, Some(&focal3));
    println!("  Players:");
    for s in &players {
        let g = s.group.map(|g| format!("G{g}")).unwrap_or_else(|| "—".into());
        println!(
            "    A{} {g}  dist={:.1} m  spd={:.2}  acc={}  dec={}  eff={:.2}",
            s.player_idx,
            s.total_distance,
            s.avg_speed,
            s.accel_count,
            s.decel_count,
            s.path_efficiency.unwrap_or(f64::NAN)
        );
    }
    let cfg3 = PhaseWindow {
        accel_threshold: 0.8,
        ..PhaseWindow::default()
    };
    if let Ok(effort) = batch3.pairwise_effort_phase(&cfg3) {
        let dir = batch3.pairwise_directional_phase(&cfg3).ok();
        let groups = batch3.groups.as_deref();
        let mut within = Vec::new();
        let mut versus = Vec::new();
        let n = batch3.num_agents();
        for i in 0..n {
            for j in (i + 1)..n {
                let ev = effort[i][j]
                    .as_ref()
                    .and_then(|p| p.event_in_phase_fraction.or(p.sign_agree_fraction))
                    .map(|f| format!("{f:.2}"))
                    .unwrap_or_else(|| "—".into());
                let dlab = dir
                    .as_ref()
                    .and_then(|m| m[i][j].as_ref())
                    .and_then(|d| d.dominant)
                    .map(|r| r.short_label())
                    .unwrap_or("—");
                let same = match groups {
                    Some(g) => g.get(i) == g.get(j),
                    None => true,
                };
                if same {
                    within.push(format!("A{i}–A{j} e={ev} d={dlab}"));
                } else {
                    versus.push(format!("A{i}×A{j} e={ev} d={dlab}"));
                }
            }
        }
        println!("  Within  {}", within.join("  "));
        println!("  Versus  {}", versus.join("  "));
    }
    for g in batch3.per_group_summaries(0.8, 1.0, Some(&focal3)) {
        println!(
            "  Team G{}  n={}  dist={:.0} m  acc={}  dec={}  effort-in={:?}  dir-in={:?}  spread={:.1} m",
            g.group,
            g.num_players,
            g.total_distance,
            g.total_accels,
            g.total_decels,
            g.mean_effort_in_phase,
            g.mean_directional_in_phase,
            g.avg_intra_group_distance
        );
    }

    println!("\nDone.");
}
