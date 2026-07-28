use ratatui::{
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
    Frame,
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
             Panels: frame details · event tags · per-player summaries.\n\n\
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

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(8),
        Constraint::Length(5),
        Constraint::Length(6),
    ])
    .split(inner);

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
        if let Some(spatial_frame) = batch.frame(idx) {
            let mut lines: Vec<String> = vec![
                format!(
                    "Frame {}/{}   t={:.2}s    [agents={}  gt={}  dec={}  focal={}]",
                    idx,
                    n_times.saturating_sub(1),
                    spatial_frame.time,
                    spatial_frame.num_agents(),
                    app.spatial_labels.as_ref().map(|l| l.len()).unwrap_or(0),
                    app.spatial_decisions.as_ref().map(|d| d.len()).unwrap_or(0),
                    focal.len()
                ),
                format!(
                    "Focal pos: ({:.1}, {:.1})",
                    spatial_frame.focal_pos().map_or(0.0, |p| p.x),
                    spatial_frame.focal_pos().map_or(0.0, |p| p.y)
                ),
            ];

            if let Some(labels) = &app.spatial_labels {
                if !labels.is_empty() {
                    if let Some(row0) = labels.first() {
                        if idx < row0.len() {
                            let g0 = &row0[idx];
                            let g1 = labels.get(1).and_then(|r| r.get(idx)).unwrap_or(g0);
                            let g2 = labels.get(2).and_then(|r| r.get(idx)).unwrap_or(g0);
                            lines.push(format!(
                                "Ground truth : {:<11}   {:<11}   {:<11}",
                                format!("{:?}", g0),
                                format!("{:?}", g1),
                                format!("{:?}", g2)
                            ));
                        }
                    }
                }
            }

            let mut current_carriers = vec![];
            if let Some(decisions) = &app.spatial_decisions {
                if !decisions.is_empty() {
                    if let Some(row0) = decisions.first() {
                        if idx < row0.len() {
                            let d0 = &row0[idx];
                            let d1 = decisions.get(1).and_then(|r| r.get(idx)).unwrap_or(d0);
                            let d2 = decisions.get(2).and_then(|r| r.get(idx)).unwrap_or(d0);
                            lines.push(format!(
                                "Classified   : {:<11}({:.2})   {:<11}({:.2})   {:<11}({:.2})",
                                format!("{:?}", d0.action),
                                d0.confidence.unwrap_or(0.0),
                                format!("{:?}", d1.action),
                                d1.confidence.unwrap_or(0.0),
                                format!("{:?}", d2.action),
                                d2.confidence.unwrap_or(0.0),
                            ));

                            current_carriers = decisions
                                .iter()
                                .enumerate()
                                .filter_map(|(ai, row)| {
                                    row.get(idx)
                                        .and_then(|d| if d.features.is_ball_carrier { Some(ai) } else { None })
                                })
                                .collect();
                            if !current_carriers.is_empty() {
                                lines.push(format!("On-ball (classifier): {:?}", current_carriers));
                            } else {
                                lines.push("On-ball (classifier): none".to_string());
                            }
                        }
                    }
                }
            }

            lines.push("Positions + features:".to_string());
            for (i, p) in spatial_frame.agent_positions.iter().enumerate() {
                let gt = app
                    .spatial_labels
                    .as_ref()
                    .and_then(|labs| labs.get(i).and_then(|row| row.get(idx)).map(|a| format!("{:?}", a)));

                let line = if let Some(decs) = &app.spatial_decisions {
                    if let Some(d) = decs.get(i).and_then(|row| row.get(idx)) {
                        let f = &d.features;
                        let mut parts: Vec<String> = vec![format!("CL:{:<11}", format!("{:?}", d.action))];
                        if let Some(c) = d.confidence {
                            parts.push(format!("conf={:.2}", c));
                        }
                        parts.push(format!("spd={:.1}", f.speed));
                        parts.push(format!("fwd={:+.2}", f.forward_component));
                        parts.push(format!("ball={}", if f.is_ball_carrier { "Y" } else { "N" }));
                        if let Some(v) = f.nearest_opponent_dist {
                            parts.push(format!("near={:.1}", v));
                        }
                        if let Some(v) = f.free_space_ahead {
                            parts.push(format!("free={:.1}", v));
                        }
                        if let Some(&fp) = focal.get(idx) {
                            let df = p.distance(fp);
                            parts.push(format!("dfoc={:.1}", df));
                        }
                        if let Some(v) = f.vel_toward_focal {
                            parts.push(format!("v2f={:+.2}", v));
                        }
                        let feats = parts.join("  ");
                        format!("  A{}: ({:5.1},{:5.1})  {}", i, p.x, p.y, feats)
                    } else if let Some(g) = gt {
                        format!("  A{}: ({:5.1},{:5.1})  GT:{}", i, p.x, p.y, g)
                    } else {
                        format!("  A{}: ({:5.1},{:5.1})", i, p.x, p.y)
                    }
                } else if let Some(g) = gt {
                    format!("  A{}: ({:5.1},{:5.1})  GT:{}", i, p.x, p.y, g)
                } else {
                    format!("  A{}: ({:5.1},{:5.1})", i, p.x, p.y)
                };
                lines.push(line);
            }

            if let Some(fpos) = focal.get(idx) {
                lines.push(format!("  Focal: ({:5.1},{:5.1})", fpos.x, fpos.y));
            } else if let Some(fpos) = spatial_frame.focal_pos() {
                lines.push(format!("  Focal: ({:5.1},{:5.1})", fpos.x, fpos.y));
            }

            let detail = Paragraph::new(lines.join("\n"));
            frame.render_widget(detail, chunks[2]);

            let mut ev_lines = vec!["Events / markers (< > or 1-9 to jump):".to_string()];
            if app.spatial_events.is_empty() {
                ev_lines.push("  (no events tagged in this demo)".into());
            } else {
                for (i, (f, desc)) in app.spatial_events.iter().enumerate().take(9) {
                    let marker = if *f == idx { "▶ " } else { "  " };
                    ev_lines.push(format!("{}{}: frame {}  {}", marker, i, f, desc));
                }
                if app.spatial_events.len() > 9 {
                    ev_lines.push("  ... (more events)".into());
                }
            }
            let events_p =
                Paragraph::new(ev_lines.join("\n")).block(Block::new().borders(Borders::TOP).title(" Event Tags "));
            frame.render_widget(events_p, chunks[3]);

            let summaries = app
                .spatial_batch
                .as_ref()
                .map(|b| b.per_player_summaries(0.8, 1.0, Some(focal)))
                .unwrap_or_default();
            let mut sum_lines = vec!["Per-agent summary (full trajectory):".to_string()];
            for s in &summaries {
                let focal_str = s
                    .avg_dist_to_focal
                    .map(|d| format!("  dfoc_avg={:.2}", d))
                    .unwrap_or_default();
                sum_lines.push(format!(
                    "  A{}: dist={:.1}  spd={:.2}  max={:.1}  acc={}  dec={}  load={:.2}{}",
                    s.player_idx,
                    s.total_distance,
                    s.avg_speed,
                    s.max_speed,
                    s.accel_count,
                    s.decel_count,
                    s.estimated_load,
                    focal_str
                ));
            }
            let sum_p =
                Paragraph::new(sum_lines.join("\n")).block(Block::new().borders(Borders::TOP).title(" Summary Data "));
            frame.render_widget(sum_p, chunks[4]);
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
