use crossterm::event::{
    KeyCode,
    KeyModifiers,
};

use crate::app::App;

pub fn handle_spatial_keys(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
    // Import menu (full-screen) — must run before viz nav
    if app.pending_spatial_import || app.spatial_view == crate::app::SpatialView::ImportData {
        match code {
            KeyCode::Char('1') | KeyCode::Char('g') | KeyCode::Char('G') => {
                app.seed_spatial_demo();
                app.pending_spatial_import = false;
                app.spatial_view = crate::app::SpatialView::Visualize;
                app.status = "Spatial: generated synthetic demo".to_string();
                return false;
            }
            KeyCode::Char('2') | KeyCode::Enter => {
                if try_load_first_spatial_csv(app) {
                    app.pending_spatial_import = false;
                    app.spatial_view = crate::app::SpatialView::Visualize;
                    return false;
                }
                app.status = "Spatial: no suitable .csv in ./data/ — press 1/g for synthetic.".to_string();
                return false;
            }
            KeyCode::Char('v') | KeyCode::Char('V') | KeyCode::Esc => {
                app.pending_spatial_import = false;
                app.spatial_view = crate::app::SpatialView::Visualize;
                app.clear_esc_quit();
                app.status = "Spatial: back to visualize".to_string();
                return false;
            }
            _ => return false, // swallow keys while menu is open
        }
    }

    // Visualize mode — top-level actions
    match code {
        // Generate synthetic immediately (single keypress)
        KeyCode::Char('g') | KeyCode::Char('G') => {
            app.seed_spatial_demo();
            app.spatial_view = crate::app::SpatialView::Visualize;
            app.pending_spatial_import = false;
            app.status = "Spatial: generated synthetic demo".to_string();
            return false;
        }
        KeyCode::Char('i') | KeyCode::Char('I') => {
            app.spatial_view = crate::app::SpatialView::ImportData;
            app.pending_spatial_import = true;
            app.status = "Spatial import: 1/g=generate  2/Enter=load csv  Esc/v=back".to_string();
            return false;
        }
        KeyCode::Char('v') | KeyCode::Char('V') => {
            app.spatial_view = crate::app::SpatialView::Visualize;
            app.pending_spatial_import = false;
            app.status = "Spatial: visualize (←→ n/p < > 1-9  g=gen  b=ball  i=import)".to_string();
            return false;
        }
        _ => {}
    }

    if let Some(batch) = &app.spatial_batch {
        let max_frame = batch.num_times().saturating_sub(1);
        match code {
            KeyCode::Left => {
                if app.spatial_frame_idx > 0 {
                    app.spatial_frame_idx -= 1;
                }
                app.status = format!("Spatial: frame {} / {}", app.spatial_frame_idx, max_frame);
            }
            KeyCode::Right => {
                if app.spatial_frame_idx < max_frame {
                    app.spatial_frame_idx += 1;
                }
                app.status = format!("Spatial: frame {} / {}", app.spatial_frame_idx, max_frame);
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                if app.spatial_frame_idx < max_frame {
                    app.spatial_frame_idx += 1;
                }
                app.status = format!("Spatial: frame {} / {}", app.spatial_frame_idx, max_frame);
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                if app.spatial_frame_idx > 0 {
                    app.spatial_frame_idx -= 1;
                }
                app.status = format!("Spatial: frame {} / {}", app.spatial_frame_idx, max_frame);
            }
            KeyCode::Char('<') => {
                if let Some((f, _)) = app
                    .spatial_events
                    .iter()
                    .rev()
                    .find(|(f, _)| *f < app.spatial_frame_idx)
                {
                    app.spatial_frame_idx = *f;
                    app.status = format!("Spatial: prev event → frame {}", app.spatial_frame_idx);
                }
            }
            KeyCode::Char('>') => {
                if let Some((f, _)) = app.spatial_events.iter().find(|(f, _)| *f > app.spatial_frame_idx) {
                    app.spatial_frame_idx = *f;
                    app.status = format!("Spatial: next event → frame {}", app.spatial_frame_idx);
                }
            }
            // Ball-carrier infer (g/i are reserved for generate/import at top level)
            KeyCode::Char('b') | KeyCode::Char('B') => {
                if let (Some(batch), Some(focal_vec)) = (&app.spatial_batch, &app.spatial_focal) {
                    let maxf = batch.num_times().saturating_sub(1);
                    let idx = app.spatial_frame_idx.min(maxf);
                    let fpos = focal_vec.get(idx).copied();
                    if let Some(carrier) = batch.infer_ball_carrier_at(idx, fpos) {
                        let extra = app
                            .spatial_decisions
                            .as_ref()
                            .and_then(|decs| decs.get(carrier).and_then(|r| r.get(idx)))
                            .map(|d| {
                                format!(
                                    " spd={:.1} fwd={:+.2} conf={:.2}",
                                    d.features.speed,
                                    d.features.forward_component,
                                    d.confidence.unwrap_or(0.0)
                                )
                            })
                            .unwrap_or_default();
                        app.status = format!("Spatial: inferred carrier = agent {}{} (frame {})", carrier, extra, idx);
                    } else {
                        app.status = format!("Spatial: no clear carrier (frame {})", idx);
                    }
                }
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                app.status = "Spatial: frame view + events + summaries (arrows n/p < > for events)".to_string();
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                // Enhanced export: CSV + JSON for LLM/agent, plus metadata file
                if let Some(batch) = &app.spatial_batch {
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let base = format!("data/spatial_export_{}", ts);

                    // CSV
                    let csv_p = format!("{}.csv", base);
                    if let Ok(mut w) = csv::Writer::from_path(&csv_p) {
                        let _ = w.write_record(["frame", "agent", "x", "y", "action", "conf", "speed", "fwd"]);
                        for f in 0..batch.num_times() {
                            if let Some(frame) = batch.frame(f) {
                                for (ai, pos) in frame.agent_positions.iter().enumerate() {
                                    let (act, conf, spd, fwd) = if let Some(decs) = &app.spatial_decisions {
                                        if let Some(d) = decs.get(ai).and_then(|r| r.get(f)) {
                                            (
                                                format!("{:?}", d.action),
                                                d.confidence.unwrap_or(0.0),
                                                d.features.speed,
                                                d.features.forward_component,
                                            )
                                        } else {
                                            ("".into(), 0.0, 0.0, 0.0)
                                        }
                                    } else {
                                        ("".into(), 0., 0., 0.)
                                    };
                                    let _ = w.write_record(&[
                                        f.to_string(),
                                        ai.to_string(),
                                        pos.x.to_string(),
                                        pos.y.to_string(),
                                        act,
                                        conf.to_string(),
                                        spd.to_string(),
                                        fwd.to_string(),
                                    ]);
                                }
                            }
                        }
                        let _ = w.flush();
                    }

                    // Simple JSON (no serde dep)
                    let json_p = format!("{}.json", base);
                    if let Ok(mut f) = std::fs::File::create(&json_p) {
                        use std::io::Write;
                        let mut json = String::from("{\n  \"frames\": [\n");
                        for f in 0..batch.num_times().min(50) {
                            // limit for size
                            json.push_str(&format!("    {{\"frame\": {}, \"agents\": [", f));
                            if let Some(frame) = batch.frame(f) {
                                for (ai, pos) in frame.agent_positions.iter().enumerate() {
                                    let act = app
                                        .spatial_decisions
                                        .as_ref()
                                        .and_then(|ds| ds.get(ai).and_then(|r| r.get(f)))
                                        .map(|d| format!("\"{:?}\"", d.action))
                                        .unwrap_or("\"\"".into());
                                    json.push_str(&format!(
                                        "{{\"id\":{},\"x\":{},\"y\":{},\"action\":{}}},",
                                        ai, pos.x, pos.y, act
                                    ));
                                }
                            }
                            json.pop(); // trailing ,
                            json.push_str("]},\n");
                        }
                        json.pop();
                        json.pop();
                        json.push_str("\n  ]\n}");
                        let _ = f.write_all(json.as_bytes());
                    }

                    // Metadata file (for LLM + structure)
                    let meta_p = format!("{}_meta.json", base);
                    if let Ok(mut f) = std::fs::File::create(&meta_p) {
                        use std::io::Write;
                        let n_agents = batch.num_times().min(1); // approx
                        let meta = format!(
                            "{{\n  \"source\": \"symview spatial export\",\n  \"num_frames\": {},\n  \"num_agents\": {},\n  \"has_decisions\": {},\n  \"export_ts\": {}\n}}",
                            batch.num_times(),
                            n_agents,
                            app.spatial_decisions.is_some(),
                            ts
                        );
                        let _ = f.write_all(meta.as_bytes());
                    }

                    app.status = format!("Exported CSV+JSON+meta → {}_*.{{csv,json}}", base);
                }
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if let Some(digit) = c.to_digit(10) {
                    let ev_idx = digit as usize;
                    if ev_idx < app.spatial_events.len() {
                        let (frame, desc) = &app.spatial_events[ev_idx];
                        app.spatial_frame_idx = *frame;
                        app.status = format!("Spatial: jumped to event {} '{}' (frame {})", ev_idx, desc, frame);
                    } else if !app.spatial_events.is_empty() {
                        app.status = format!("Spatial: no event {} (have 0-{})", ev_idx, app.spatial_events.len() - 1);
                    }
                }
            }
            KeyCode::Esc => {
                // SpatialSym root: Esc-Esc quits. (Legacy BioSym jump to Dynamics avoided.)
                return app.esc_root_or_quit();
            }
            _ => {}
        }
    } else {
        app.status = "Spatial: NO BATCH (seed may have failed)".to_string();
    }

    if app.status.is_empty() {
        let maxf = app
            .spatial_batch
            .as_ref()
            .map(|b| b.num_times().saturating_sub(1))
            .unwrap_or(0);
        let _ev_hint = if !app.spatial_events.is_empty() {
            " | <>/1-9 events"
        } else {
            ""
        };
        app.status = format!("Spatial: frame {}/{}", app.spatial_frame_idx, maxf);
    }

    false
}

pub fn try_load_first_spatial_csv(app: &mut App) -> bool {
    let data_dir = std::path::Path::new("data");
    if !data_dir.exists() {
        return false;
    }

    if let Ok(entries) = std::fs::read_dir(data_dir) {
        let mut candidates = Vec::new();
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                // support per-match dir: look inside for csv
                if let Ok(subs) = std::fs::read_dir(&p) {
                    for se in subs.flatten() {
                        let sp = se.path();
                        if let Some(e) = sp.extension().and_then(|x| x.to_str()) {
                            if e.eq_ignore_ascii_case("csv") {
                                candidates.push(sp);
                            }
                        }
                    }
                }
            } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("csv") {
                    candidates.push(p);
                }
            }
        }
        for p in candidates {
            if let Ok(content) = std::fs::read_to_string(&p) {
                if content
                    .lines()
                    .next()
                    .is_some_and(|h| h.to_lowercase().contains("agent_id"))
                    && app.load_spatial_csv(&p).is_ok()
                {
                    return true;
                }
            }
        }
    }
    false
}
