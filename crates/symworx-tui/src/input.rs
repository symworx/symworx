use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::{App, Tab};

pub fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    if code == KeyCode::Char('q') {
        return true;
    }

    if code == KeyCode::Char('?') && modifiers.contains(KeyModifiers::ALT) {
        app.help_mode = !app.help_mode;
        return false;
    }

    // Refresh must be reliable (even in submodes / while typing) — early return per conventions
    if (code == KeyCode::Char('r') && modifiers.contains(KeyModifiers::CONTROL))
        || code == KeyCode::F(5)
    {
        app.refresh_file_list();
        app.status = "Refreshed file list (Ctrl+R / F5)".to_string();
        app.ensure_status_for_current_tab();
        return false;
    }

    if app.help_mode {
        if code == KeyCode::Esc {
            app.help_mode = false;
            return false;
        }
        // Still allow global home navigation from help dashboard
        if (code == KeyCode::Char('h') || code == KeyCode::Char('H'))
            && modifiers.contains(KeyModifiers::CONTROL)
        {
            app.help_mode = false;
            app.clear_submodes();
            app.switch_workflow(crate::app::Workflow::Home);
            return false;
        }
        if code == KeyCode::Char('0') && !modifiers.contains(KeyModifiers::CONTROL) {
            app.help_mode = false;
            if app.current_workflow != crate::app::Workflow::Home {
                app.clear_submodes();
                app.switch_workflow(crate::app::Workflow::Home);
            }
            return false;
        }
        // allow re-toggle help
        if code == KeyCode::Char('?') && modifiers.contains(KeyModifiers::ALT) {
            app.help_mode = false;
            return false;
        }
        return false;
    }

    // Home / workflow selector access (always available, early)
    if (code == KeyCode::Char('h') || code == KeyCode::Char('H'))
        && modifiers.contains(KeyModifiers::CONTROL)
    {
        app.clear_submodes();
        app.switch_workflow(crate::app::Workflow::Home);
        return false;
    }
    // Allow quick home from number 0 when appropriate (or always map)
    if code == KeyCode::Char('0') && !modifiers.contains(KeyModifiers::CONTROL) {
        if app.current_workflow != crate::app::Workflow::Home {
            app.clear_submodes();
            app.switch_workflow(crate::app::Workflow::Home);
        }
        return false;
    }

    match code {
        KeyCode::Char('1') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_tab = Tab::Import;
            app.current_workflow = crate::app::Workflow::BioSym;
            app.ensure_status_for_current_tab();
            return false;
        }
        KeyCode::Char('2') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_tab = Tab::Explore;
            app.current_workflow = crate::app::Workflow::BioSym;
            app.ensure_status_for_current_tab();
            return false;
        }
        KeyCode::Char('3') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_tab = Tab::Dynamics;
            app.current_workflow = crate::app::Workflow::BioSym;
            app.ensure_status_for_current_tab();
            return false;
        }
        KeyCode::Char('4') if modifiers.contains(KeyModifiers::CONTROL) => {
            // Note: from BioSym we prefer not mixing, but quick switch is allowed via Ctrl+4
            app.current_tab = Tab::Spatial;
            app.current_workflow = crate::app::Workflow::SpatialSym;
            app.ensure_status_for_current_tab();
            return false;
        }
        KeyCode::Left if modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_workflow == crate::app::Workflow::Home {
                return false;
            }
            // Scope navigation to parent workflow's subtabs (per main path order: 1=BioSym, 2=LoadSym, 3=SpatialSym)
            app.current_tab = match app.current_workflow {
                crate::app::Workflow::BioSym => match app.current_tab {
                    Tab::Import => Tab::Import,
                    Tab::Explore => Tab::Import,
                    Tab::Dynamics => Tab::Explore,
                    _ => Tab::Import,
                },
                crate::app::Workflow::SpatialSym => Tab::Spatial,
                crate::app::Workflow::LoadSym => Tab::LoadSym,
                _ => app.current_tab,
            };
            app.ensure_status_for_current_tab();
            return false;
        }
        KeyCode::Right if modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_workflow == crate::app::Workflow::Home {
                return false;
            }
            // Scope navigation to parent workflow's subtabs (BioSym does not reach SpatialSym/LoadSym)
            app.current_tab = match app.current_workflow {
                crate::app::Workflow::BioSym => match app.current_tab {
                    Tab::Import => Tab::Explore,
                    Tab::Explore => Tab::Dynamics,
                    Tab::Dynamics => Tab::Dynamics,
                    _ => Tab::Import,
                },
                crate::app::Workflow::SpatialSym => Tab::Spatial,
                crate::app::Workflow::LoadSym => Tab::LoadSym,
                _ => app.current_tab,
            };
            app.ensure_status_for_current_tab();
            return false;
        }
        KeyCode::Char('g') | KeyCode::Char('G') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_workflow = crate::app::Workflow::BioSym;
            if app.current_tab != Tab::Import {
                app.current_tab = Tab::Import;
            }
            app.pending_generate = true;
            app.manual_path.clear();
            app.file_filter.clear();
            app.status =
                "BioSym demo data: 1 = Resting PPG   2 = Respiration   3 = Stride   Esc = cancel"
                    .to_string();
            app.ensure_status_for_current_tab();
            return false;
        }
        _ => {}
    }

    // Route Home first (landing selector takes precedence for its keys)
    if app.current_workflow == crate::app::Workflow::Home || app.current_tab == Tab::Home {
        return handle_home_keys(app, code, modifiers);
    }

    match app.current_tab {
        Tab::Import => handle_import_keys(app, code, modifiers),
        Tab::Explore => handle_explore_keys(app, code, modifiers),
        Tab::Dynamics => handle_dynamics_keys(app, code),
        Tab::Spatial => handle_spatial_keys(app, code, modifiers),
        Tab::LoadSym => handle_loadsym_keys(app, code, modifiers),
        Tab::Home => handle_home_keys(app, code, modifiers),
    }
}

fn handle_spatial_keys(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
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
                app.status =
                    "Spatial: no suitable .csv in ./data/ — press 1/g for synthetic.".to_string();
                return false;
            }
            KeyCode::Char('v') | KeyCode::Char('V') | KeyCode::Esc => {
                app.pending_spatial_import = false;
                app.spatial_view = crate::app::SpatialView::Visualize;
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
                if let Some((f, _)) = app
                    .spatial_events
                    .iter()
                    .find(|(f, _)| *f > app.spatial_frame_idx)
                {
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
                        app.status = format!(
                            "Spatial: inferred carrier = agent {}{} (frame {})",
                            carrier, extra, idx
                        );
                    } else {
                        app.status = format!("Spatial: no clear carrier (frame {})", idx);
                    }
                }
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                app.status = "Spatial: frame view + events + summaries (arrows n/p < > for events)"
                    .to_string();
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
                        let _ = w.write_record([
                            "frame", "agent", "x", "y", "action", "conf", "speed", "fwd",
                        ]);
                        for f in 0..batch.num_times() {
                            if let Some(frame) = batch.frame(f) {
                                for (ai, pos) in frame.agent_positions.iter().enumerate() {
                                    let (act, conf, spd, fwd) =
                                        if let Some(decs) = &app.spatial_decisions {
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
                        let meta = format!("{{\n  \"source\": \"symview spatial export\",\n  \"num_frames\": {},\n  \"num_agents\": {},\n  \"has_decisions\": {},\n  \"export_ts\": {}\n}}", batch.num_times(), n_agents, app.spatial_decisions.is_some(), ts);
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
                        app.status = format!(
                            "Spatial: jumped to event {} '{}' (frame {})",
                            ev_idx, desc, frame
                        );
                    } else if !app.spatial_events.is_empty() {
                        app.status = format!(
                            "Spatial: no event {} (have 0-{})",
                            ev_idx,
                            app.spatial_events.len() - 1
                        );
                    }
                }
            }
            KeyCode::Esc => {
                app.current_tab = Tab::Dynamics;
                app.status = "Back to Dynamics".to_string();
                app.ensure_status_for_current_tab();
                return false;
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

fn handle_import_keys(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
    if app.pending_generate {
        match code {
            KeyCode::Char('1') => {
                if let Err(e) = crate::processing::generate_demo_and_load(
                    app,
                    crate::generate::DemoPreset::RestingPPG,
                ) {
                    app.status = format!("Generation failed: {e}");
                }
                app.pending_generate = false;
                app.manual_path.clear();
                return false;
            }
            KeyCode::Char('2') => {
                if let Err(e) = crate::processing::generate_demo_and_load(
                    app,
                    crate::generate::DemoPreset::LightRespiration,
                ) {
                    app.status = format!("Generation failed: {e}");
                }
                app.pending_generate = false;
                app.manual_path.clear();
                return false;
            }
            KeyCode::Char('3') => {
                if let Err(e) = crate::processing::generate_demo_and_load(
                    app,
                    crate::generate::DemoPreset::SimpleStride,
                ) {
                    app.status = format!("Generation failed: {e}");
                }
                app.pending_generate = false;
                app.manual_path.clear();
                return false;
            }
            KeyCode::Esc => {
                app.pending_generate = false;
                app.status = "BioSym generate cancelled".to_string();
                return false;
            }
            _ => {}
        }
        return false;
    }

    if app.filter_mode {
        match code {
            KeyCode::Char(c) if c.is_ascii() => {
                app.file_filter.push(c);
                app.ensure_valid_selection();
            }
            KeyCode::Backspace => {
                app.file_filter.pop();
                app.ensure_valid_selection();
            }
            KeyCode::Enter | KeyCode::Esc => {
                app.filter_mode = false;
            }
            _ => {}
        }
        return false;
    }

    match code {
        KeyCode::Char('/') => {
            app.filter_mode = true;
            app.file_filter.clear();
            app.ensure_valid_selection();
            return false;
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if let Some(path) = app.selected_path().cloned() {
                if let Err(e) = crate::convert::parquet_to_csv(&path, &path.with_extension("csv")) {
                    app.status = format!("Convert failed: {e}");
                } else {
                    app.status = format!("Converted → {}", path.display());
                    app.refresh_file_list();
                }
            }
            return false;
        }
        KeyCode::Enter => {
            if let Err(e) = app.load_selected_or_manual() {
                app.status = format!("Load error: {e}");
            }
            return false;
        }
        KeyCode::Up => {
            app.select_prev();
            return false;
        }
        KeyCode::Down => {
            app.select_next();
            return false;
        }
        _ => {}
    }

    // Generic char handling for manual path entry (after sub-mode checks, per input priority rules)
    match code {
        KeyCode::Char(c) if c.is_ascii() && !c.is_control() => {
            app.manual_path.push(c);
            return false;
        }
        KeyCode::Backspace => {
            app.manual_path.pop();
            return false;
        }
        KeyCode::Esc => {
            if !app.manual_path.is_empty() {
                app.manual_path.clear();
                return false;
            }
        }
        _ => {}
    }

    false
}

fn handle_explore_keys(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
    if app.pending_process {
        // IMPORTANT: check submode first (per AGENTS.md input priority rules)
        match code {
            KeyCode::Char('1') => {
                app.process_selection = 0;
                app.status = "Process: Moving Average selected. ↑↓ select   ←→/± window   Enter apply   Esc cancel".to_string();
            }
            KeyCode::Char('2') => {
                app.process_selection = 1;
                app.status = "Process: Median Filter selected. ↑↓ select   ←→/± window   Enter apply   Esc cancel".to_string();
            }
            KeyCode::Char('3') => {
                app.process_selection = 2;
                app.status =
                    "Process: Detrend selected. ↑↓ select   ←→/± window   Enter apply   Esc cancel"
                        .to_string();
            }
            KeyCode::Up => {
                if app.process_selection > 0 {
                    app.process_selection -= 1;
                } else {
                    app.process_selection = 2;
                }
                app.status = format!(
                    "Process: {} selected (↑↓ to change)",
                    ["Moving Average", "Median Filter", "Detrend (mean)"][app.process_selection]
                );
            }
            KeyCode::Down => {
                app.process_selection = (app.process_selection + 1) % 3;
                app.status = format!(
                    "Process: {} selected (↑↓ to change)",
                    ["Moving Average", "Median Filter", "Detrend (mean)"][app.process_selection]
                );
            }
            KeyCode::Enter => {
                if let Some(signal) = &mut app.loaded_signal {
                    let window = app.process_window;
                    let processed = match app.process_selection {
                        0 => crate::processing::moving_average(&signal.current, window),
                        1 => crate::processing::median_filter(&signal.current, window),
                        2 => crate::processing::detrend_mean(&signal.current),
                        _ => signal.current.clone(),
                    };
                    signal.apply_processed(processed);
                    app.status = "Processing applied.".to_string();
                }
                app.pending_process = false;
            }
            KeyCode::Esc => {
                app.pending_process = false;
                app.status = "Process cancelled.".to_string();
            }
            KeyCode::Left | KeyCode::Char('-') | KeyCode::Char('_') => {
                if app.process_window > 1 {
                    app.process_window -= 1;
                }
                app.status = format!("Process window: {}", app.process_window);
            }
            KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('=') => {
                app.process_window += 1;
                app.status = format!("Process window: {}", app.process_window);
            }
            _ => {}
        }
        return false;
    }

    match code {
        KeyCode::Esc => {
            // Back to Import (BioSym file list / generate)
            app.pending_process = false;
            app.current_tab = Tab::Import;
            app.current_workflow = crate::app::Workflow::BioSym;
            app.status = "Import — file list / Ctrl+G generate".to_string();
            app.ensure_status_for_current_tab();
            return false;
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            app.pending_process = true;
            app.status =
                "Process: ↑↓ select 1/2/3   ←→/± window   Enter apply   Esc cancel".to_string();
            return false;
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if let Some(signal) = &mut app.loaded_signal {
                signal.reset();
                app.explore_scroll = 0;
                app.status = "Reset to original.".to_string();
            }
            return false;
        }
        // Pan x-axis viewport for long BioSym signals
        KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
            if app.loaded_signal.is_some() {
                let step = 30usize;
                app.explore_scroll = app.explore_scroll.saturating_sub(step);
                app.status = format!("Explore: pan x → start={}", app.explore_scroll);
            }
            return false;
        }
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
            if let Some(sig) = &app.loaded_signal {
                let view_len = crate::ui::tabs::explore::EXPLORE_VIEW_LEN;
                let max_start = sig.current.len().saturating_sub(view_len);
                let step = 30usize;
                app.explore_scroll = (app.explore_scroll + step).min(max_start);
                app.status = format!(
                    "Explore: pan x → start={} (max {})",
                    app.explore_scroll, max_start
                );
            }
            return false;
        }
        _ => {}
    }
    false
}

fn handle_dynamics_keys(app: &mut App, code: KeyCode) -> bool {
    if app.pending_rqa {
        match code {
            KeyCode::Left | KeyCode::Char('-') | KeyCode::Char('_') => {
                if app.rqa_params.radius > 0.05 {
                    app.rqa_params.radius -= 0.05;
                }
                app.status = format!("RQA/cRQA radius: {:.2}", app.rqa_params.radius);
            }
            KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('=') => {
                app.rqa_params.radius += 0.05;
                app.status = format!("RQA/cRQA radius: {:.2}", app.rqa_params.radius);
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                app.rqa_params.m = (app.rqa_params.m % 8) + 1;
                app.status = format!("RQA m (dim): {}", app.rqa_params.m);
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                app.rqa_params.tau = (app.rqa_params.tau % 5) + 1;
                app.status = format!("RQA tau (delay): {}", app.rqa_params.tau);
            }
            KeyCode::Enter => {
                if let Some(sig) = &app.loaded_signal {
                    let res = symworx_dynamics::rqa(
                        &sig.current,
                        app.rqa_params.m,
                        app.rqa_params.tau,
                        app.rqa_params.radius,
                        app.rqa_params.theiler,
                    );
                    // also store RP for updated viz
                    let rp = symworx_dynamics::RecurrencePlot::from_series(
                        &sig.current,
                        app.rqa_params.m,
                        app.rqa_params.tau,
                        app.rqa_params.radius,
                        app.rqa_params.theiler,
                    );
                    app.last_rqa = Some(res);
                    app.last_rp = Some(rp);
                    app.last_crqa = None;
                    app.status =
                        "RQA computed. See Dynamics tab (improved RP preview + MSE).".to_string();
                } else {
                    app.status = "Load signal first (Import/Explore).".to_string();
                }
                app.pending_rqa = false;
            }
            KeyCode::Esc => {
                app.pending_rqa = false;
                app.status = "RQA param edit cancelled".to_string();
            }
            _ => {}
        }
        return false;
    }

    match code {
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.pending_rqa = true;
            app.status = format!("RQA params: m={} tau={} rad={:.2}  ←→/± rad  m/t  Enter=compute (RQA)  Esc", app.rqa_params.m, app.rqa_params.tau, app.rqa_params.radius);
            return false;
        }
        KeyCode::Char('x') | KeyCode::Char('X') => {
            // cRQA: prefer reference vs current; fallback to current vs time-reversed for demo
            if let Some(sig) = &app.loaded_signal {
                let (name_a, series_a, series_b) = if let Some((ref_name, ref_data)) = &app.reference_series {
                    (ref_name.clone(), ref_data.clone(), sig.current.clone())
                } else {
                    // fallback demo: signal vs its reverse (shows asymmetry/structure differences)
                    let rev: Vec<f64> = sig.current.iter().rev().copied().collect();
                    ("current".to_string(), sig.current.clone(), rev)
                };
                let res = symworx_dynamics::crqa(
                    &series_a,
                    &series_b,
                    app.rqa_params.m,
                    app.rqa_params.tau,
                    app.rqa_params.radius,
                    app.rqa_params.theiler,
                );
                app.last_crqa = Some(res);
                app.last_rqa = None; // focus display on the cross result
                app.status = format!("cRQA computed ({} vs other). See Dynamics tab.", name_a);
            } else {
                app.status = "Load signal first for cRQA.".to_string();
            }
            return false;
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            // Pin current as reference series for subsequent cRQA
            if let Some(sig) = &app.loaded_signal {
                app.reference_series = Some((sig.name.clone(), sig.current.clone()));
                app.status = format!("Pinned '{}' as cRQA reference. Press x to cross with current (or after processing).", sig.name);
            } else {
                app.status = "No signal to pin.".to_string();
            }
            return false;
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            app.rqa_params = crate::app::RqaParams::default();
            app.last_rqa = None;
            app.last_rp = None;
            app.last_crqa = None;
            // leave reference_series (user can 'p' again or we could clear with another key)
            app.status = "RQA params + results reset (ref kept; 'p' to change)".to_string();
            return false;
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            if let Some(r) = app.last_crqa.as_ref().or(app.last_rqa.as_ref()) {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let path = format!("data/crqa_export_{}.csv", ts); // works for both
                if let Err(e) = export_rqa_csv(&path, r, &app.rqa_params) {
                    app.status = format!("Export failed: {}", e);
                } else {
                    app.status = format!("Exported metrics → {} (RQA or cRQA)", path);
                }
            } else {
                app.status = "Compute RQA (c) or cRQA (x) first.".to_string();
            }
            return false;
        }
        _ => {}
    }
    false
}

fn export_rqa_csv(
    path: &str,
    r: &symworx_dynamics::RqaResult,
    params: &crate::app::RqaParams,
) -> anyhow::Result<()> {
    use std::{fs::File, io::Write};
    let mut f = File::create(path)?;
    writeln!(f, "m,tau,radius,theiler")?;
    writeln!(
        f,
        "{},{},{},{}",
        params.m, params.tau, params.radius, params.theiler
    )?;
    writeln!(
        f,
        "recurrence_rate,determinism,laminarity,lmax,lmean,lentr,trapping_time,vmax,n_recurrences"
    )?;
    writeln!(
        f,
        "{:.6},{:.6},{:.6},{},{:.4},{:.4},{:.4},{},{}",
        r.recurrence_rate,
        r.determinism,
        r.laminarity,
        r.lmax,
        r.lmean,
        r.lentr,
        r.trapping_time,
        r.vmax,
        r.n_recurrences
    )?;
    Ok(())
}

fn handle_home_keys(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
    // Arrow and digit selection for paths on landing
    match code {
        KeyCode::Up => {
            if app.home_selection > 0 {
                app.home_selection -= 1;
            }
            return false;
        }
        KeyCode::Down => {
            if app.home_selection < 2 {
                app.home_selection += 1;
            }
            return false;
        }
        KeyCode::Char('1') => {
            app.switch_workflow(crate::app::Workflow::BioSym);
            return false;
        }
        KeyCode::Char('2') => {
            app.switch_workflow(crate::app::Workflow::LoadSym);
            return false;
        }
        KeyCode::Char('3') => {
            app.switch_workflow(crate::app::Workflow::SpatialSym);
            return false;
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if app.home_selection == 1 {
                app.status =
                    "LoadSym: select from its own tab (1/2/3 views now implemented).".to_string();
            }
            return false;
        }
        KeyCode::Char('i') | KeyCode::Char('I') => {
            if app.home_selection == 1 {
                app.status =
                    "LoadSym: import via activity files (FIT via symworx-io) — see LoadSym tab."
                        .to_string();
            } else if app.home_selection == 2 {
                // convenience for spatial too
                app.switch_workflow(crate::app::Workflow::SpatialSym);
            }
            return false;
        }
        KeyCode::Enter => {
            match app.home_selection {
                0 => app.switch_workflow(crate::app::Workflow::BioSym),
                1 => app.switch_workflow(crate::app::Workflow::LoadSym),
                2 => app.switch_workflow(crate::app::Workflow::SpatialSym),
                _ => {}
            }
            return false;
        }
        KeyCode::Esc => {
            // On home, Esc does nothing special (or could quit but q is for that)
            return false;
        }
        _ => {}
    }
    false
}

fn handle_loadsym_keys(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
    // In list view: arrow/digit selection of sub view
    if app.loadsym_view == crate::app::LoadSymView::List {
        match code {
            KeyCode::Up => {
                if app.loadsym_selection > 0 {
                    app.loadsym_selection -= 1;
                }
                return false;
            }
            KeyCode::Down => {
                if app.loadsym_selection < 2 {
                    app.loadsym_selection += 1;
                }
                return false;
            }
            KeyCode::Char('1') => {
                app.loadsym_view = crate::app::LoadSymView::Workout;
                app.status =
                    "LoadSym: Workout Analysis (peaks, best efforts, threshold bars) — Esc back"
                        .to_string();
                return false;
            }
            KeyCode::Char('2') => {
                app.loadsym_view = crate::app::LoadSymView::Calendar;
                app.loadsym_scroll = 0;
                app.status = "LoadSym: Calendar — ↑↓/←→ scroll days  • Esc: list".to_string();
                return false;
            }
            KeyCode::Char('3') => {
                app.loadsym_view = crate::app::LoadSymView::Optimization;
                app.status = "LoadSym: Programming Optimization — Esc to list".to_string();
                return false;
            }
            KeyCode::Enter => {
                match app.loadsym_selection {
                    0 => app.loadsym_view = crate::app::LoadSymView::Workout,
                    1 => {
                        app.loadsym_view = crate::app::LoadSymView::Calendar;
                        app.loadsym_scroll = 0;
                    }
                    2 => app.loadsym_view = crate::app::LoadSymView::Optimization,
                    _ => {}
                }
                return false;
            }
            KeyCode::Char('g') | KeyCode::Char('G') => {
                // explicit synthetic demo generation (not default)
                app.daily_loads =
                    symworx_loadsym::load::generate_demo_daily_loads(14, 400.0, 100.0);
                app.status = "LoadSym: generated synthetic demo daily loads (use 'i' to import real activity data)".to_string();
                return false;
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                if let Some(act) = try_load_first_activity() {
                    app.loaded_activity = Some(act.clone());
                    app.activity_scroll = 0;
                    app.activity_series = 0;
                    app.workout_user_thresh = 0.0;
                    app.workout_user_min_dur = 3;
                    app.loadsym_view = crate::app::LoadSymView::Workout;
                    app.status = format!("Loaded activity into Workout: {}", act.source);
                } else {
                    app.status = "No activity file in ./data/. Put .fit or headered .csv there and press i again.".to_string();
                }
                return false;
            }
            KeyCode::Esc => {
                // already in list, perhaps switch workflow back? keep simple
                return false;
            }
            _ => {}
        }
        return false;
    }

    // Sub-view specific
    match app.loadsym_view {
        crate::app::LoadSymView::Workout => {
            match code {
                KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
                    let scroll = if app.loaded_activity.is_some() {
                        &mut app.activity_scroll
                    } else {
                        &mut app.loadsym_scroll
                    };
                    if *scroll > 0 {
                        *scroll -= 10;
                    } // page scroll for long traces
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    let scroll = if app.loaded_activity.is_some() {
                        &mut app.activity_scroll
                    } else {
                        &mut app.loadsym_scroll
                    };
                    *scroll += 10;
                }
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    // Try to load a .fit or activity file from ./data
                    if let Some(act) = try_load_first_activity() {
                        let n = act.times_s.len();
                        let name = act
                            .source
                            .split('/')
                            .last()
                            .unwrap_or(&act.source)
                            .to_string();
                        app.loaded_activity = Some(act);
                        app.activity_scroll = 0;
                        app.activity_series = 0;
                        app.workout_user_thresh = 0.0;
                        app.workout_user_min_dur = 3;
                        app.status =
                            format!("Loaded {} — {} samples. 1/2/3=series  ←→ scroll", name, n);
                    } else {
                        app.status = "No .fit/.csv in ./data/. Drop Garmin/Polar/SRM file (or CSV with headers).".to_string();
                    }
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    app.loaded_activity = None;
                    app.activity_scroll = 0;
                    app.activity_series = 0;
                    app.workout_user_thresh = 0.0;
                    app.workout_user_min_dur = 3;
                    app.status =
                        "Cleared loaded activity + user thresh. Using demo series.".to_string();
                }
                KeyCode::Char('1') => {
                    app.activity_series = 0;
                    app.status = "Switched to Power series (if available)".to_string();
                }
                KeyCode::Char('2') => {
                    app.activity_series = 1;
                    app.status = "Switched to Heart Rate series (if available)".to_string();
                }
                KeyCode::Char('3') => {
                    app.activity_series = 2;
                    app.status = "Switched to Speed series (if available)".to_string();
                }
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    app.status =
                        "Exported workout (CSV to data/ would be written here).".to_string();
                }
                // True exploration: user-defined threshold + min duration (samples for regions)
                KeyCode::Char('t') => {
                    app.workout_user_thresh = (app.workout_user_thresh + 5.0).max(0.0);
                    app.status = format!(
                        "User thresh set to {:.1} (auto when 0)",
                        app.workout_user_thresh
                    );
                }
                KeyCode::Char('T') => {
                    app.workout_user_thresh = (app.workout_user_thresh - 5.0).max(0.0);
                    app.status = format!(
                        "User thresh set to {:.1} (auto when 0)",
                        app.workout_user_thresh
                    );
                }
                KeyCode::Char('d') => {
                    app.workout_user_min_dur = app.workout_user_min_dur.saturating_add(1);
                    app.status = format!(
                        "User min_dur: {} (reset with r or set 0 for auto)",
                        app.workout_user_min_dur
                    );
                }
                KeyCode::Char('D') => {
                    if app.workout_user_min_dur > 1 {
                        app.workout_user_min_dur -= 1;
                    }
                    app.status = format!(
                        "User min_dur: {} (reset with r or set 0 for auto)",
                        app.workout_user_min_dur
                    );
                }
                // FTP adjust for TSS/NP/IF
                KeyCode::Char('f') => {
                    app.ftp = (app.ftp + 5.0).max(50.0);
                    app.status = format!("FTP set to {:.0} W (affects NP/TSS)", app.ftp);
                }
                KeyCode::Char('F') => {
                    app.ftp = (app.ftp - 5.0).max(50.0);
                    app.status = format!("FTP set to {:.0} W (affects NP/TSS)", app.ftp);
                }
                KeyCode::Esc => {
                    app.loadsym_view = crate::app::LoadSymView::List;
                    app.status = "LoadSym — back to list".to_string();
                }
                _ => {}
            }
        }
        crate::app::LoadSymView::Calendar => {
            match code {
                KeyCode::Left | KeyCode::Char('h') => {
                    if app.loadsym_scroll > 0 {
                        app.loadsym_scroll -= 1;
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    app.loadsym_scroll =
                        (app.loadsym_scroll + 1).min(app.daily_loads.len().saturating_sub(1));
                }
                KeyCode::Up => {
                    if app.loadsym_scroll > 0 {
                        app.loadsym_scroll -= 1;
                    }
                }
                KeyCode::Down => {
                    app.loadsym_scroll =
                        (app.loadsym_scroll + 1).min(app.daily_loads.len().saturating_sub(1));
                }
                KeyCode::Esc => {
                    app.loadsym_view = crate::app::LoadSymView::List;
                    app.status = "LoadSym — back to list".to_string();
                }
                _ => {}
            }
            app.status = format!(
                "Calendar: day {} / {}",
                app.loadsym_scroll,
                app.daily_loads.len()
            );
        }
        crate::app::LoadSymView::Optimization => {
            if code == KeyCode::Esc {
                app.loadsym_view = crate::app::LoadSymView::List;
                app.status = "LoadSym — back to list".to_string();
            }
        }
        _ => {}
    }
    false
}

/// Best-effort loader: finds first *.fit / *.csv in several likely locations
/// (data/, rides/, training/, and loadsym_archive awareness).
fn try_load_first_activity() -> Option<symworx_io::ActivityData> {
    let candidates = ["data", "rides", "training", "archive", "."];
    for base in &candidates {
        let dir = std::path::Path::new(base);
        if !dir.exists() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    continue;
                } // simple, no recurse for now
                if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                    let extl = ext.to_lowercase();
                    if matches!(extl.as_str(), "fit" | "csv" | "txt") {
                        if let Ok(act) = symworx_io::load_activity(&p.to_string_lossy()) {
                            if !act.times_s.is_empty() {
                                return Some(act);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Try to load a real spatial trajectories CSV from ./data/
fn try_load_first_spatial_csv(app: &mut App) -> bool {
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
                    .map_or(false, |h| h.to_lowercase().contains("agent_id"))
                {
                    if app.load_spatial_csv(&p).is_ok() {
                        return true;
                    }
                }
            }
        }
    }
    false
}
