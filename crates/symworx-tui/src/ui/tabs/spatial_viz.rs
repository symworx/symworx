// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Spatial Visualize graphics: plan-view canvas and pair-phase strip.

use ratatui::{
    Frame,
    layout::Rect,
    style::{
        Color,
        Style,
    },
    symbols,
    text::{
        Line,
        Span,
    },
    widgets::{
        Axis,
        Block,
        Borders,
        Chart,
        Dataset,
        GraphType,
        canvas::{
            Canvas,
            Circle,
            Line as CanvasLine,
            Points,
            Rectangle,
        },
    },
};

use crate::app::App;

const TRAIL_FRAMES: usize = 8;
/// C(6,2) = 15 covers a 3v3; extra pairs are dropped.
const PAIR_CAP: usize = 15;

/// G0 (attack) white; G1 (defend) magenta — avoids cyan/yellow/red used for phase edges.
pub fn team_color(group: Option<u32>) -> Color {
    match group {
        Some(1) => Color::LightMagenta,
        _ => Color::White,
    }
}

fn group_of(batch: &symworx_spatialsym::AgentTrajectories, i: usize) -> Option<u32> {
    batch.groups.as_ref().and_then(|g| g.get(i).copied())
}

fn focused_agent_idx(app: Option<&App>, batch: &symworx_spatialsym::AgentTrajectories, idx: usize) -> usize {
    if let Some(app) = app
        && let Some(decs) = &app.spatial_decisions
    {
        for (i, row) in decs.iter().enumerate() {
            if row.get(idx).is_some_and(|d| d.features.is_ball_carrier) {
                return i;
            }
        }
    }
    0.min(batch.num_agents().saturating_sub(1))
}

/// Same gates as the Summary `Now` line.
pub fn now_phase_cfg() -> symworx_spatialsym::PhaseWindow {
    symworx_spatialsym::PhaseWindow {
        accel_threshold: 0.8,
        window_sec: 1.0,
        ..symworx_spatialsym::PhaseWindow::default()
    }
}

fn edge_color(dominant: Option<symworx_spatialsym::DirectionalRelation>) -> Color {
    match dominant {
        Some(symworx_spatialsym::DirectionalRelation::InPhase) => Color::Cyan,
        Some(symworx_spatialsym::DirectionalRelation::SpatiallyOpposed) => Color::Yellow,
        Some(symworx_spatialsym::DirectionalRelation::OutOfPhase) => Color::Red,
        Some(symworx_spatialsym::DirectionalRelation::MixedOutOfPhase) => Color::Magenta,
        None => Color::DarkGray,
    }
}

/// World (x along length, y across) → canvas (across, along) so attack (+x) is up.
fn attack_up(x: f64, y: f64) -> (f64, f64) {
    (y, x)
}

fn attack_up_pts(pts: &[(f64, f64)]) -> Vec<(f64, f64)> {
    pts.iter().map(|&(x, y)| attack_up(x, y)).collect()
}

fn padded_bounds(xs: &[f64], ys: &[f64]) -> ([f64; 2], [f64; 2]) {
    let (mut xmin, mut xmax) = match (xs.iter().copied().reduce(f64::min), xs.iter().copied().reduce(f64::max)) {
        (Some(a), Some(b)) => (a, b),
        _ => return ([-1.0, 1.0], [-1.0, 1.0]),
    };
    let (mut ymin, mut ymax) = match (ys.iter().copied().reduce(f64::min), ys.iter().copied().reduce(f64::max)) {
        (Some(a), Some(b)) => (a, b),
        _ => return ([-1.0, 1.0], [-1.0, 1.0]),
    };
    if (xmax - xmin).abs() < 1e-6 {
        xmin -= 1.0;
        xmax += 1.0;
    }
    if (ymax - ymin).abs() < 1e-6 {
        ymin -= 1.0;
        ymax += 1.0;
    }
    let px = (xmax - xmin) * 0.10;
    let py = (ymax - ymin) * 0.10;
    ([xmin - px, xmax + px], [ymin - py, ymax + py])
}

/// Bird's-eye view of the current frame (auto-fit to data, not the full pitch).
pub fn render_spatial_plan(
    frame: &mut Frame,
    app: &App,
    batch: &symworx_spatialsym::AgentTrajectories,
    focal: &[symworx_spatialsym::Point2],
    idx: usize,
    area: Rect,
) {
    let t = batch.times.get(idx).copied().unwrap_or(0.0);
    let trail_start = idx.saturating_sub(TRAIL_FRAMES.saturating_sub(1));

    let mut xs = Vec::new();
    let mut ys = Vec::new();
    let mut trails: Vec<Vec<(f64, f64)>> = Vec::new();
    for agent in &batch.positions {
        let mut trail = Vec::new();
        for t_i in trail_start..=idx {
            if let Some(p) = agent.get(t_i) {
                trail.push((p.x, p.y));
                xs.push(p.x);
                ys.push(p.y);
            }
        }
        trails.push(trail);
    }
    if let Some(fp) = focal.get(idx) {
        xs.push(fp.x);
        ys.push(fp.y);
    }

    let focus = focused_agent_idx(Some(app), batch, idx);
    let mut path_pts: Vec<(f64, f64)> = Vec::new();
    if let Some(agent) = batch.positions.get(focus) {
        for p in agent.iter().take(idx + 1) {
            path_pts.push((p.x, p.y));
            xs.push(p.x);
            ys.push(p.y);
        }
    }
    let chord = if path_pts.len() >= 2 {
        Some((path_pts[0], *path_pts.last().unwrap()))
    } else {
        None
    };

    let field = batch.playing_dimensions.map(|d| d.bounds());
    let rotate = field.is_some();
    let (x_bounds, y_bounds) = if let Some((xmin, xmax, ymin, ymax)) = field {
        let pad_x = (ymax - ymin) * 0.02;
        let pad_y = (xmax - xmin) * 0.02;
        // Canvas X = field y (width); canvas Y = field x (length, attack up).
        ([ymin - pad_x, ymax + pad_x], [xmin - pad_y, xmax + pad_y])
    } else {
        padded_bounds(&xs, &ys)
    };

    let trails: Vec<Vec<(f64, f64)>> = if rotate {
        trails.iter().map(|t| attack_up_pts(t)).collect()
    } else {
        trails
    };
    let path_pts = if rotate { attack_up_pts(&path_pts) } else { path_pts };
    let chord = chord.map(|((x1, y1), (x2, y2))| {
        if rotate {
            (attack_up(x1, y1), attack_up(x2, y2))
        } else {
            ((x1, y1), (x2, y2))
        }
    });

    let cfg = now_phase_cfg();
    let effort = batch.pairwise_effort_phase_at(idx, &cfg).ok();
    let dir = batch.pairwise_directional_phase_at(idx, &cfg).ok();
    let n = batch.num_agents();
    let mut edges: Vec<(f64, f64, f64, f64, Color)> = Vec::new();
    if let Some(frame_pos) = batch.frame(idx) {
        let mut shown = 0usize;
        for i in 0..n {
            for j in (i + 1)..n {
                if shown >= PAIR_CAP {
                    break;
                }
                let pi = frame_pos.agent_positions.get(i);
                let pj = frame_pos.agent_positions.get(j);
                if let (Some(a), Some(b)) = (pi, pj) {
                    let has = effort
                        .as_ref()
                        .and_then(|m| m.get(i).and_then(|row| row.get(j)))
                        .and_then(|o| o.as_ref())
                        .is_some();
                    let dom = dir
                        .as_ref()
                        .and_then(|m| m.get(i).and_then(|row| row.get(j)))
                        .and_then(|o| o.as_ref())
                        .and_then(|d| d.dominant);
                    let color = if has || dom.is_some() {
                        edge_color(dom)
                    } else {
                        Color::DarkGray
                    };
                    let (x1, y1, x2, y2) = if rotate {
                        let (cx1, cy1) = attack_up(a.x, a.y);
                        let (cx2, cy2) = attack_up(b.x, b.y);
                        (cx1, cy1, cx2, cy2)
                    } else {
                        (a.x, a.y, b.x, b.y)
                    };
                    edges.push((x1, y1, x2, y2, color));
                    shown += 1;
                }
            }
        }
    }

    let agents: Vec<(f64, f64, String, Color)> = batch
        .frame(idx)
        .map(|f| {
            f.agent_positions
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let (x, y) = if rotate { attack_up(p.x, p.y) } else { (p.x, p.y) };
                    (x, y, format!("A{i}"), team_color(group_of(batch, i)))
                })
                .collect()
        })
        .unwrap_or_default();
    let g0_pts: Vec<(f64, f64)> = agents
        .iter()
        .filter(|(_, _, _, c)| *c == Color::White)
        .map(|(x, y, _, _)| (*x, *y))
        .collect();
    let g1_pts: Vec<(f64, f64)> = agents
        .iter()
        .filter(|(_, _, _, c)| *c == Color::LightMagenta)
        .map(|(x, y, _, _)| (*x, *y))
        .collect();

    let mut mark_rects: Vec<Rectangle> = Vec::new();
    let mut mark_lines: Vec<CanvasLine> = Vec::new();
    let mut mark_spots: Vec<(f64, f64)> = Vec::new();
    let mut mark_circles: Vec<Circle> = Vec::new();
    if let (Some(dims), Some(marks)) = (batch.playing_dimensions, batch.play_area_markings) {
        for plus_x in [true, false] {
            for inner in [false, true] {
                let (x, y, w, h) = marks.end_box_rect(dims, plus_x, inner);
                let (cx, cy, cw, ch) = if rotate { (y, x, h, w) } else { (x, y, w, h) };
                mark_rects.push(Rectangle {
                    x: cx,
                    y: cy,
                    width: cw,
                    height: ch,
                    color: Color::DarkGray,
                });
            }
            let (ga, gb) = marks.goal_segment(dims, plus_x);
            let (x1, y1) = if rotate { attack_up(ga.x, ga.y) } else { (ga.x, ga.y) };
            let (x2, y2) = if rotate { attack_up(gb.x, gb.y) } else { (gb.x, gb.y) };
            mark_lines.push(CanvasLine {
                x1,
                y1,
                x2,
                y2,
                color: Color::Gray,
            });
            let spot = marks.penalty_spot(dims, plus_x);
            mark_spots.push(if rotate {
                attack_up(spot.x, spot.y)
            } else {
                (spot.x, spot.y)
            });
        }
        let (cx, cy) = if rotate { attack_up(0.0, 0.0) } else { (0.0, 0.0) };
        mark_circles.push(Circle {
            x: cx,
            y: cy,
            radius: marks.center_circle.radius_m,
            color: Color::DarkGray,
        });
    }
    let focal_pt = focal
        .get(idx)
        .map(|p| if rotate { attack_up(p.x, p.y) } else { (p.x, p.y) });

    let field_label = batch
        .playing_dimensions
        .map(|d| format!("{:.0}×{:.0} m", d.length_m, d.width_m))
        .unwrap_or_else(|| "auto".into());

    let canvas = Canvas::default()
        .block(Block::new().borders(Borders::TOP).title(format!(
            " Plan  {field_label}  attack ↑  t={t:.2}s  A{focus} path/chord "
        )))
        .x_bounds(x_bounds)
        .y_bounds(y_bounds)
        .marker(symbols::Marker::Braille)
        .paint(move |ctx| {
            if let Some((xmin, xmax, ymin, ymax)) = field {
                // Rotated: canvas X = field y, canvas Y = field x.
                ctx.draw(&Rectangle {
                    x: ymin,
                    y: xmin,
                    width: ymax - ymin,
                    height: xmax - xmin,
                    color: Color::DarkGray,
                });
                ctx.draw(&CanvasLine {
                    x1: ymin,
                    y1: 0.0,
                    x2: ymax,
                    y2: 0.0,
                    color: Color::DarkGray,
                });
                ctx.draw(&CanvasLine {
                    x1: 0.0,
                    y1: xmin,
                    x2: 0.0,
                    y2: xmax,
                    color: Color::DarkGray,
                });
                ctx.layer();
            }
            for r in &mark_rects {
                ctx.draw(r);
            }
            for ln in &mark_lines {
                ctx.draw(ln);
            }
            if !mark_spots.is_empty() {
                ctx.draw(&Points {
                    coords: &mark_spots,
                    color: Color::Gray,
                });
            }
            for c in &mark_circles {
                ctx.draw(c);
            }
            ctx.layer();
            for trail in &trails {
                if trail.len() >= 2 {
                    ctx.draw(&Points {
                        coords: trail,
                        color: Color::DarkGray,
                    });
                }
            }
            ctx.layer();
            if path_pts.len() >= 2 {
                for w in path_pts.windows(2) {
                    ctx.draw(&CanvasLine {
                        x1: w[0].0,
                        y1: w[0].1,
                        x2: w[1].0,
                        y2: w[1].1,
                        color: Color::Blue,
                    });
                }
            }
            if let Some(((x1, y1), (x2, y2))) = chord {
                ctx.draw(&CanvasLine {
                    x1,
                    y1,
                    x2,
                    y2,
                    color: Color::LightGreen,
                });
            }
            ctx.layer();
            for &(x1, y1, x2, y2, color) in &edges {
                ctx.draw(&CanvasLine { x1, y1, x2, y2, color });
            }
            ctx.layer();
            if !g0_pts.is_empty() {
                ctx.draw(&Points {
                    coords: &g0_pts,
                    color: Color::White,
                });
            }
            if !g1_pts.is_empty() {
                ctx.draw(&Points {
                    coords: &g1_pts,
                    color: Color::LightMagenta,
                });
            }
            if let Some((fx, fy)) = focal_pt {
                ctx.draw(&Points {
                    coords: &[(fx, fy)],
                    color: Color::LightYellow,
                });
                ctx.print(fx, fy, Line::from("+"));
            }
            for (x, y, label, color) in &agents {
                ctx.print(
                    *x,
                    *y,
                    Line::from(Span::styled(label.clone(), Style::default().fg(*color))),
                );
            }
        });
    frame.render_widget(canvas, area);
}

/// Effort in-phase series for A0–A1 with a playhead at the current frame.
pub fn render_pair_strip(frame: &mut Frame, batch: &symworx_spatialsym::AgentTrajectories, idx: usize, area: Rect) {
    let title = " A0–A1 effort-in  (←→ frame) ";
    if batch.num_agents() < 2 {
        frame.render_widget(
            ratatui::widgets::Paragraph::new("Need ≥2 agents for pair strip")
                .block(Block::new().borders(Borders::TOP).title(title)),
            area,
        );
        return;
    }
    let cfg = now_phase_cfg();
    let series = batch.pairwise_effort_phase_series(0, 1, &cfg).unwrap_or_default();
    let mut data: Vec<(f64, f64)> = Vec::new();
    for (i, score) in series.iter().enumerate() {
        if let Some(y) = score
            .as_ref()
            .and_then(|s| s.event_in_phase_fraction.or(s.sign_agree_fraction))
        {
            data.push((i as f64, y));
        }
    }
    let x_hi = (series.len().max(2) - 1) as f64;
    let play_idx = symworx_spatialsym::accel_index_for_frame(idx);
    let play_y = play_idx.and_then(|i| {
        series
            .get(i)
            .and_then(|s| s.as_ref())
            .and_then(|s| s.event_in_phase_fraction.or(s.sign_agree_fraction))
    });
    let playhead: Vec<(f64, f64)> = match (play_idx, play_y) {
        (Some(i), Some(y)) => vec![(i as f64, y)],
        _ => Vec::new(),
    };

    if data.is_empty() {
        frame.render_widget(
            ratatui::widgets::Paragraph::new("no comparable effort samples")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::new().borders(Borders::TOP).title(title)),
            area,
        );
        return;
    }

    let mut datasets = vec![
        Dataset::default()
            .name("effort-in")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&data),
    ];
    if !playhead.is_empty() {
        datasets.push(
            Dataset::default()
                .name("now")
                .marker(symbols::Marker::Dot)
                .graph_type(GraphType::Scatter)
                .style(Style::default().fg(Color::Yellow))
                .data(&playhead),
        );
    }
    let chart = Chart::new(datasets)
        .block(Block::new().borders(Borders::TOP).title(title))
        .x_axis(
            Axis::default()
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, x_hi])
                .labels(vec![Line::from("start"), Line::from("end")]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, 1.0])
                .labels(vec![Line::from("0"), Line::from("1")]),
        );
    frame.render_widget(chart, area);
}

/// One compact feature line per agent (no coordinates — those live on the plan).
pub fn compact_agent_lines(app: &App, idx: usize, focal: &[symworx_spatialsym::Point2]) -> Vec<String> {
    let mut lines = Vec::new();
    let Some(batch) = app.spatial_batch.as_ref() else {
        return lines;
    };
    let Some(frame) = batch.frame(idx) else {
        return lines;
    };
    let focus = focused_agent_idx(Some(app), batch, idx);
    let mut last_g: Option<Option<u32>> = None;
    for (i, p) in frame.agent_positions.iter().enumerate() {
        let g = group_of(batch, i);
        if last_g != Some(g) {
            lines.push(match g {
                Some(0) => "  G0 attack (white)".into(),
                Some(1) => "  G1 defend (magenta)".into(),
                Some(n) => format!("  G{n}"),
                None => "  agents".into(),
            });
            last_g = Some(g);
        }
        let mark = if i == focus { "*" } else { " " };
        if let Some(d) = app
            .spatial_decisions
            .as_ref()
            .and_then(|decs| decs.get(i).and_then(|row| row.get(idx)))
        {
            let f = &d.features;
            let mut parts = vec![
                format!("{mark}A{i}"),
                format!("({:.0},{:.0})", p.x, p.y),
                format!("CL:{:?}", d.action),
                format!("spd={:.1}", f.speed),
                format!("ball={}", if f.is_ball_carrier { "Y" } else { "N" }),
            ];
            if let Some(v) = f.nearest_opponent_dist {
                parts.push(format!("near={v:.1}"));
            }
            if let Some(&fp) = focal.get(idx) {
                parts.push(format!("dfoc={:.1}", p.distance(fp)));
            }
            lines.push(format!("  {}", parts.join("  ")));
        } else {
            lines.push(format!("  {mark}A{i}  ({:.0},{:.0})", p.x, p.y));
        }
    }
    lines
}
