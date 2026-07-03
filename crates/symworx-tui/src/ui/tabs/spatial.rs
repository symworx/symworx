use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};
use crate::app::App;

pub fn render_spatial_tab(frame: &mut Frame, app: &App, area: Rect) {
    if app.help_mode {
        let help = Paragraph::new(
            "Spatial tab help (M-? or Esc to close)\n\n\
             Frame navigation (minimal set):\n\
             • ← →   : step frames\n\
             • n / p : next / previous frame\n\
             • < / > : jump to previous / next event tag\n\
             • 1-9   : direct jump to event N\n\n\
             Actions:\n\
             • g     : regenerate the demo\n\
             • i     : infer ball carrier using current frame\n\
             • l     : refresh status/legend\n\n\
             The sections below always show:\n\
             - current frame details (GT vs classified + features)\n\
             - list of event tags (with current marker)\n\
             - summary stats from per_player_summaries"
        ).block(Block::new().borders(Borders::ALL).title(" Help — Spatial "));
        frame.render_widget(help, area);
        return;
    }

    let outer = Block::new()
        .title(" SpatialSym (trajectories, decisions, space use) ")
        .borders(Borders::ALL)
        .border_style(Color::Cyan);

    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    // Sub-tab / view header (equivalent of sub-tabs inside this domain)
    let sub_header = Paragraph::new(format!(
        "  [g] Generate/Synth   [i] Import matches/games (placeholder)   [v] Visualize   current: {:?}",
        app.spatial_view
    ))
    .style(Style::default().fg(Color::Yellow));
    // Allocate small top chunk
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(8),
        Constraint::Length(5),
        Constraint::Length(6),
    ])
    .split(inner);

    frame.render_widget(sub_header, chunks[0]);

    // If in import/generate sub-view, render a compact menu + placeholder list
    if app.spatial_view != crate::app::SpatialView::Visualize || app.pending_spatial_import {
        let import_text = "Spatial Import / Generate\n\n\
            1 : Regenerate current synthetic demo\n\
            2 / i : Load placeholder match or game (stub — populates viz with demo data)\n\
            Enter or numbers : act   Esc : back to viz\n\n\
            Future: select real .csv (time,agent_id,x,y) — uses symworx-spatialsym::load_trajectories_csv\n\
            Different sports/matches will appear here (placeholder entries below):\n\
            • demo_match_2026.csv (synthetic)\n\
            • import_game_soccer_01 (stub)\n\
            • example_cross_session (placeholder)";
        let p = Paragraph::new(import_text).block(Block::new().borders(Borders::ALL).title(" Import / Generate "));
        frame.render_widget(p, chunks[1]);
        // Skip normal viz chunks when showing import UI
        return;
    }

    let nav_text = if app.spatial_batch.is_some() {
        let n_ev = app.spatial_events.len();
        let ev_hint = if n_ev > 0 {
            format!(" | 1-{}: jump event  < > for events", n_ev.min(9))
        } else {
            "".into()
        };
        format!(
            "Frame nav: ←→  n/p    g:regen{}   | Legend: conf spd fwd ball near free dfoc v2f + Creation/Conversion/Prevention (M-? for details)",
            ev_hint
        )
    } else {
        "Spatial tab — synthetic demo. Press g or i to enter generate/import. Then use ←→ or n/p to move frames.".to_string()
    };
    let nav = Paragraph::new(nav_text)
        .style(Style::default().fg(Color::DarkGray));
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
                    if let Some(row0) = labels.get(0) {
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
                    if let Some(row0) = decisions.get(0) {
                        if idx < row0.len() {
                            let d0 = &row0[idx];
                            let d1 = decisions.get(1).and_then(|r| r.get(idx)).unwrap_or(d0);
                            let d2 = decisions.get(2).and_then(|r| r.get(idx)).unwrap_or(d0);
                            lines.push(format!(
                                "Classified   : {:<11}({:.2})   {:<11}({:.2})   {:<11}({:.2})",
                                format!("{:?}", d0.action), d0.confidence.unwrap_or(0.0),
                                format!("{:?}", d1.action), d1.confidence.unwrap_or(0.0),
                                format!("{:?}", d2.action), d2.confidence.unwrap_or(0.0),
                            ));

                            current_carriers = decisions.iter().enumerate()
                                .filter_map(|(ai, row)| row.get(idx).and_then(|d|
                                    if d.features.is_ball_carrier { Some(ai) } else { None }))
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
                let gt = app.spatial_labels.as_ref().and_then(|labs|
                    labs.get(i).and_then(|row| row.get(idx)).map(|a| format!("{:?}", a)));

                let line = if let Some(decs) = &app.spatial_decisions {
                    if let Some(d) = decs.get(i).and_then(|row| row.get(idx)) {
                        let f = &d.features;
                        let mut parts: Vec<String> = vec![
                            format!("CL:{:<11}", format!("{:?}", d.action)),
                        ];
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
            let events_p = Paragraph::new(ev_lines.join("\n"))
                .block(Block::new().borders(Borders::TOP).title(" Event Tags "));
            frame.render_widget(events_p, chunks[3]);

            let summaries = app.spatial_batch.as_ref().map(|b| b.per_player_summaries(0.8, 1.0, Some(focal))).unwrap_or_default();
            let mut sum_lines = vec!["Per-agent summary (full trajectory):".to_string()];
            for s in &summaries {
                let focal_str = s.avg_dist_to_focal
                    .map(|d| format!("  dfoc_avg={:.2}", d))
                    .unwrap_or_default();
                sum_lines.push(format!(
                    "  A{}: dist={:.1}  spd={:.2}  max={:.1}  acc={}  dec={}  load={:.2}{}",
                    s.player_idx, s.total_distance, s.avg_speed, s.max_speed,
                    s.accel_count, s.decel_count, s.estimated_load, focal_str
                ));
            }
            let sum_p = Paragraph::new(sum_lines.join("\n"))
                .block(Block::new().borders(Borders::TOP).title(" Summary Data "));
            frame.render_widget(sum_p, chunks[4]);

        } else {
            let content = Paragraph::new("No frame data");
            frame.render_widget(content, chunks[2]);
        }
    } else {
        let help = Paragraph::new(
            "No spatial data loaded.\n\n\
             Press 'g' or 'i' to enter generate/import sub-view.\n\
             Frame nav (Spatial):\n\
             • ← / →   : step frames\n\
             • n / p   : next / prev frame\n\
             • < / >   : prev / next event tag\n\
             • 1-9     : jump to event tag\n\
             • M-?     : this help (Alt+?)\n\
             • g/i/v   : sub views\n\n\
             Other tabs have their own M-? help."
        );
        frame.render_widget(help, chunks[2]);
    }
}
