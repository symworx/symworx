use ratatui::{
    Frame,
    layout::{
        Constraint,
        Layout,
        Rect,
    },
    style::{
        Color,
        Style,
    },
    widgets::{
        Block,
        Borders,
        Padding,
        Paragraph,
    },
};

use crate::app::App;

pub fn render_spatial_tab(frame: &mut Frame, app: &App, area: Rect) {
    if app.help_mode {
        let help = Paragraph::new(
            "SpatialSym — trajectories, decisions, space\n\
             Close help:  Esc  or  Alt-?\n\n\
             \n\
             DATA\n\n\
               g                   generate synthetic demo\n\
               i                   import menu (CSV under ./data)\n\
               v  /  Esc           leave import menu → visualize\n\n\
             \n\
             FRAMES\n\n\
               ← →                 step frame\n\
               n / p               next / previous frame\n\
               < / >               previous / next event tag\n\
               1–9                 jump to event N\n\n\
             \n\
             ACTIONS  (batch loaded)\n\n\
               b                   infer ball carrier (current frame)\n\
               e                   export CSV + JSON + meta → data/\n\
               l                   refresh status / legend\n\n\
             Panels: plan view · A0–A1 effort strip · compact agents · events · summaries.\n\
             Layout: stats left, field right (attack +x is up). 105×68 m when generated.\n\
             Pair edges: cyan=in yellow=opp red=out.\n\
             Focused agent (on-ball, else A0): blue path, green start→now chord (matches eff/rms).\n\n\
             \n\
             GLOBAL\n\n\
               Ctrl+H              Home\n\
               Esc Esc / Ctrl+Q    quit (Esc-Esc at roots only)\n",
        )
        .block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(" Help — Spatial "),
        );
        frame.render_widget(help, area);
        return;
    }

    let outer = Block::new()
        .title(" SpatialSym (trajectories, decisions, space use) ")
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1))
        .border_style(Color::Cyan);

    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    // Import / generate menu uses the full inner area (was Length(1) — looked broken)
    if app.pending_spatial_import || app.spatial_view == crate::app::SpatialView::ImportData {
        render_spatial_import_menu(frame, app, inner);
        return;
    }

    // Sub-tab header
    let sub_header = Paragraph::new(format!(
        "  [g] Generate   [i] Import   [v] Visualize   {:?}",
        app.spatial_view
    ))
    .style(Style::default().fg(Color::Yellow));

    let chunks = Layout::vertical([Constraint::Length(1), Constraint::Length(1), Constraint::Min(12)]).split(inner);

    frame.render_widget(sub_header, chunks[0]);

    let nav_text = if app.spatial_batch.is_some() {
        let n_ev = app.spatial_events.len();
        let ev_hint = if n_ev > 0 {
            format!(" | 1-{}: jump event  < > for events", n_ev.min(9))
        } else {
            "".into()
        };
        format!(
            "Frame: ←→ n/p    g:regen  i:import  b:ball{}   | conf spd fwd ball near free dfoc v2f",
            ev_hint
        )
    } else {
        "No data — press g to generate synthetic demo, or i to import CSV from ./data/.".to_string()
    };
    let nav = Paragraph::new(nav_text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(nav, chunks[1]);

    if let (Some(batch), Some(focal)) = (&app.spatial_batch, &app.spatial_focal) {
        let n_times = batch.num_times();
        let idx = app.spatial_frame_idx.min(n_times.saturating_sub(1));
        if batch.frame(idx).is_some() {
            let cols = Layout::horizontal([Constraint::Percentage(38), Constraint::Percentage(62)]).split(chunks[2]);
            let left = Layout::vertical([
                Constraint::Length(5),
                Constraint::Min(7),
                Constraint::Length(5),
                Constraint::Min(8),
            ])
            .split(cols[0]);

            super::spatial_viz::render_pair_strip(frame, batch, idx, left[0]);

            let mut agent_lines = vec![format!(
                "Frame {}/{}  t={:.2}s",
                idx,
                n_times.saturating_sub(1),
                batch.times.get(idx).copied().unwrap_or(0.0)
            )];
            agent_lines.extend(super::spatial_viz::compact_agent_lines(app, idx, focal));
            let agents_p =
                Paragraph::new(agent_lines.join("\n")).block(Block::new().borders(Borders::TOP).title(" Agents "));
            frame.render_widget(agents_p, left[1]);

            let mut ev_lines = vec!["Events (< > or 1-9):".to_string()];
            if app.spatial_events.is_empty() {
                ev_lines.push("  (no events tagged)".into());
            } else {
                for (i, (f, desc)) in app.spatial_events.iter().enumerate().take(8) {
                    let marker = if *f == idx { "▶ " } else { "  " };
                    ev_lines.push(format!("{}{}: f{} {}", marker, i, f, desc));
                }
            }
            let events_p =
                Paragraph::new(ev_lines.join("\n")).block(Block::new().borders(Borders::TOP).title(" Event Tags "));
            frame.render_widget(events_p, left[2]);

            let sum_lines = format_spatial_summaries(batch, focal, idx);
            let sum_p =
                Paragraph::new(sum_lines.join("\n")).block(Block::new().borders(Borders::TOP).title(" Summary Data "));
            frame.render_widget(sum_p, left[3]);

            super::spatial_viz::render_spatial_plan(frame, app, batch, focal, idx, cols[1]);
        } else {
            let content = Paragraph::new("No frame data");
            frame.render_widget(content, chunks[2]);
        }
    } else {
        let help = Paragraph::new(
            "No spatial data loaded.\n\n\
             • g  — generate synthetic demo (pass / press sequence)\n\
             • i  — import CSV from ./data/ (time,agent_id,x,y)\n\
             • M-? — full help\n\n\
             After data loads: ←→ n/p frames, < > events, b ball carrier, e export.",
        );
        frame.render_widget(help, chunks[2]);
    }
}

fn render_spatial_import_menu(frame: &mut Frame, app: &App, area: Rect) {
    let header = Paragraph::new(format!(
        "  [g] Generate   [i] Import   [v] Visualize   {:?}  (import menu)",
        app.spatial_view
    ))
    .style(Style::default().fg(Color::Yellow));

    let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(area);
    frame.render_widget(header, chunks[0]);

    let mut files: Vec<String> = vec![];
    if let Ok(entries) = std::fs::read_dir("data") {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_file() {
                if let Some(ext) = p.extension().and_then(|x| x.to_str()) {
                    if ext.eq_ignore_ascii_case("csv") {
                        files.push(p.file_name().unwrap_or_default().to_string_lossy().to_string());
                    }
                }
            } else if p.is_dir() {
                if let Ok(sub) = std::fs::read_dir(&p) {
                    for se in sub.flatten() {
                        if let Some(e2) = se.path().extension().and_then(|x| x.to_str()) {
                            if e2.eq_ignore_ascii_case("csv") {
                                files.push(format!(
                                    "{}/{}",
                                    p.file_name().unwrap_or_default().to_string_lossy(),
                                    se.file_name().to_string_lossy()
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    let file_list = if files.is_empty() {
        "  (no .csv found in ./data/ or subdirs)".to_string()
    } else {
        files
            .iter()
            .take(12)
            .enumerate()
            .map(|(i, f)| format!("  {}: {}", i + 1, f))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let import_text = format!(
        "Spatial Import / Generate\n\n\
         Keys:\n\
           1 / g     Generate synthetic demo (pass → press sequence)\n\
           2 / Enter Load first suitable .csv from ./data/\n\
           Esc / v   Back to visualize\n\n\
         Discovered in ./data/:\n{}\n\n\
         CSV layout: time,agent_id,x,y  |  optional match subdirs\n\
         After load/gen: ←→ frames, b=ball carrier, e=export",
        file_list
    );
    let p = Paragraph::new(import_text).block(
        Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .title(" Import / Generate "),
    );
    frame.render_widget(p, chunks[1]);
}

fn format_spatial_summaries(
    batch: &symworx_spatialsym::AgentTrajectories,
    focal: &[symworx_spatialsym::Point2],
    frame_idx: usize,
) -> Vec<String> {
    const THRESH: f64 = 0.8;
    let summaries = batch.per_player_summaries(THRESH, 1.0, Some(focal));
    let mut lines = vec!["Per-agent summary (full trajectory):".to_string()];
    for s in &summaries {
        let focal_str = s
            .avg_dist_to_focal
            .map(|d| format!("  dfoc={:.2}", d))
            .unwrap_or_default();
        let eff_str = s
            .path_efficiency
            .map(|e| format!("  eff={:.2}", e))
            .unwrap_or_else(|| "  eff=—".into());
        let rms_str = s
            .path_rms_dev_m
            .map(|d| format!("  rms={:.2}", d))
            .unwrap_or_else(|| "  rms=—".into());
        lines.push(format!(
            "  A{}: dist={:.1}  spd={:.2}  max={:.1}  acc={}  dec={}  load={:.2}{}{}{}",
            s.player_idx,
            s.total_distance,
            s.avg_speed,
            s.max_speed,
            s.accel_count,
            s.decel_count,
            s.estimated_load,
            eff_str,
            rms_str,
            focal_str
        ));
    }

    let cfg = symworx_spatialsym::PhaseWindow {
        accel_threshold: THRESH,
        ..symworx_spatialsym::PhaseWindow::default()
    };
    if let Ok(effort) = batch.pairwise_effort_phase(&cfg) {
        let dir = batch.pairwise_directional_phase(&cfg).ok();
        let n = batch.num_agents();
        let mut pair_bits = Vec::new();
        let mut shown = 0usize;
        for i in 0..n {
            for j in (i + 1)..n {
                if shown >= 6 {
                    break;
                }
                let ev = effort[i][j]
                    .as_ref()
                    .and_then(|p| p.event_in_phase_fraction.or(p.sign_agree_fraction))
                    .map(|f| format!("{:.2}", f))
                    .unwrap_or_else(|| "—".into());
                let dlab = dir
                    .as_ref()
                    .and_then(|m| m[i][j].as_ref())
                    .and_then(|d| d.dominant)
                    .map(|r| r.short_label())
                    .unwrap_or("—");
                pair_bits.push(format!("A{}–A{} e={} d={}", i, j, ev, dlab));
                shown += 1;
            }
        }
        if !pair_bits.is_empty() {
            lines.push(format!("  Pairs  {}", pair_bits.join("  ")));
        }
    }

    let now_cfg = super::spatial_viz::now_phase_cfg();
    if let Ok(now_e) = batch.pairwise_effort_phase_at(frame_idx, &now_cfg) {
        let now_d = batch.pairwise_directional_phase_at(frame_idx, &now_cfg).ok();
        let now_c = batch.pairwise_closing_at(frame_idx, &now_cfg).ok();
        let n = batch.num_agents();
        let mut bits = Vec::new();
        let mut shown = 0usize;
        for i in 0..n {
            for j in (i + 1)..n {
                if shown >= 6 {
                    break;
                }
                let ev = now_e[i][j]
                    .as_ref()
                    .and_then(|p| p.event_in_phase_fraction.or(p.sign_agree_fraction))
                    .map(|f| format!("{:.2}", f))
                    .unwrap_or_else(|| "—".into());
                let dlab = now_d
                    .as_ref()
                    .and_then(|m| m[i][j].as_ref())
                    .and_then(|d| d.dominant)
                    .map(|r| r.short_label())
                    .unwrap_or("—");
                let clab = now_c
                    .as_ref()
                    .and_then(|m| m[i][j].as_ref())
                    .and_then(|c| c.mean_i_toward_j)
                    .map(|v| format!("{:+.2}", v))
                    .unwrap_or_else(|| "—".into());
                bits.push(format!("A{}–A{} e={} d={} c={}", i, j, ev, dlab, clab));
                shown += 1;
            }
        }
        if !bits.is_empty() {
            lines.push(format!("  Now    {}", bits.join("  ")));
        }
    }

    let groups = batch.per_group_summaries(THRESH, 1.0, Some(focal));
    for g in groups {
        let e = g
            .mean_effort_in_phase
            .map(|f| format!("{:.2}", f))
            .unwrap_or_else(|| "—".into());
        let d = g
            .mean_directional_in_phase
            .map(|f| format!("{:.2}", f))
            .unwrap_or_else(|| "—".into());
        lines.push(format!(
            "  Group {}  n={}  effort-in={}  dir-in={}",
            g.group, g.num_players, e, d
        ));
    }
    lines
}
