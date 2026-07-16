use crossterm::event::{
    KeyCode,
    KeyModifiers,
};

use crate::app::{
    App,
    Tab,
};

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

const PROCESS_NAMES: [&str; 5] = [
    "Moving Average",
    "Median Filter",
    "Detrend (mean)",
    "1st derivative",
    "2nd derivative",
];

fn handle_explore_keys(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
    // Peak-parameter editor: checked before process and generic keys (input priority).
    if app.pending_peak_params {
        let n = crate::app::PeakDetectParams::N_FIELDS;
        match code {
            KeyCode::Esc => {
                app.pending_peak_params = false;
                app.status = "Peak params closed (overlays keep last detection).".to_string();
            }
            KeyCode::Up => {
                if app.peak_param_selection > 0 {
                    app.peak_param_selection -= 1;
                } else {
                    app.peak_param_selection = n - 1;
                }
                app.status = format!(
                    "Peak param: {}  (←→ live  k re-run  Enter apply+close  d defaults  Esc)",
                    crate::app::PeakDetectParams::field_name(app.peak_param_selection)
                );
            }
            KeyCode::Down => {
                app.peak_param_selection = (app.peak_param_selection + 1) % n;
                app.status = format!(
                    "Peak param: {}  (←→ live  k re-run  Enter apply+close  d defaults  Esc)",
                    crate::app::PeakDetectParams::field_name(app.peak_param_selection)
                );
            }
            KeyCode::Left | KeyCode::Char('-') | KeyCode::Char('_') => {
                if app.peak_params.nudge(app.peak_param_selection, false) {
                    app.status = crate::processing::run_peak_detection(app);
                }
            }
            KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('=') => {
                if app.peak_params.nudge(app.peak_param_selection, true) {
                    app.status = crate::processing::run_peak_detection(app);
                }
            }
            KeyCode::Char('1') => {
                app.peak_param_selection = 0;
                app.status = "Peak param: height_frac".to_string();
            }
            KeyCode::Char('2') => {
                app.peak_param_selection = 1;
                app.status = "Peak param: prom_frac".to_string();
            }
            KeyCode::Char('3') => {
                app.peak_param_selection = 2;
                app.status = "Peak param: min_interval_sec".to_string();
            }
            KeyCode::Char('4') => {
                app.peak_param_selection = 3;
                app.status = "Peak param: match_tol".to_string();
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                let kind = app
                    .loaded_signal
                    .as_ref()
                    .map(|s| s.kind)
                    .unwrap_or_default();
                app.peak_params = crate::app::PeakDetectParams::for_kind(kind);
                app.status = format!(
                    "Peak params reset to {} defaults. {}",
                    kind.label(),
                    crate::processing::run_peak_detection(app)
                );
            }
            // k/K: re-run while staying in the editor (chart below updates live).
            KeyCode::Char('k') | KeyCode::Char('K') => {
                let msg = crate::processing::run_peak_detection(app);
                app.status = format!("Re-ran peak detect — {}", msg);
            }
            // Enter: apply (re-run) and close — same pattern as process menu.
            // Also accept \n/\r for terminals that emit Char instead of KeyCode::Enter.
            KeyCode::Enter | KeyCode::Char('\n') | KeyCode::Char('\r') => {
                let msg = crate::processing::run_peak_detection(app);
                app.pending_peak_params = false;
                app.status = format!("Peak params applied — {}", msg);
            }
            _ => {}
        }
        return false;
    }

    if app.pending_process {
        // IMPORTANT: check submode first (per AGENTS.md input priority rules)
        let n_ops = PROCESS_NAMES.len();
        match code {
            KeyCode::Char('1') => {
                app.process_selection = 0;
                app.status = format!(
                    "Process: {} selected. ↑↓ select   ←→/± window   Enter apply   Esc cancel",
                    PROCESS_NAMES[0]
                );
            }
            KeyCode::Char('2') => {
                app.process_selection = 1;
                app.status = format!(
                    "Process: {} selected. ↑↓ select   ←→/± window   Enter apply   Esc cancel",
                    PROCESS_NAMES[1]
                );
            }
            KeyCode::Char('3') => {
                app.process_selection = 2;
                app.status = format!(
                    "Process: {} selected. ↑↓ select   ←→/± window   Enter apply   Esc cancel",
                    PROCESS_NAMES[2]
                );
            }
            KeyCode::Char('4') => {
                app.process_selection = 3;
                app.status = format!(
                    "Process: {} selected (no window). Enter apply   Esc cancel",
                    PROCESS_NAMES[3]
                );
            }
            KeyCode::Char('5') => {
                app.process_selection = 4;
                app.status = format!(
                    "Process: {} selected (no window). Enter apply   Esc cancel",
                    PROCESS_NAMES[4]
                );
            }
            KeyCode::Up => {
                if app.process_selection > 0 {
                    app.process_selection -= 1;
                } else {
                    app.process_selection = n_ops - 1;
                }
                app.status = format!(
                    "Process: {} selected (↑↓ to change)",
                    PROCESS_NAMES[app.process_selection]
                );
            }
            KeyCode::Down => {
                app.process_selection = (app.process_selection + 1) % n_ops;
                app.status = format!(
                    "Process: {} selected (↑↓ to change)",
                    PROCESS_NAMES[app.process_selection]
                );
            }
            KeyCode::Enter => {
                let label = PROCESS_NAMES[app.process_selection.min(n_ops - 1)];
                let window = app.process_window;
                let sel = app.process_selection;
                if let Some(signal) = &mut app.loaded_signal {
                    let processed = match sel {
                        0 => crate::processing::moving_average(&signal.current, window),
                        1 => crate::processing::median_filter(&signal.current, window),
                        2 => crate::processing::detrend_mean(&signal.current),
                        3 => crate::processing::first_derivative(&signal.current),
                        4 => crate::processing::second_derivative(&signal.current),
                        _ => signal.current.clone(),
                    };
                    signal.apply_processed(processed);
                }
                // Re-run peak detect so parameter + processing outcomes stay visible.
                let peak_msg = crate::processing::run_peak_detection(app);
                app.status = format!("{} applied. {}", label, peak_msg);
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
            app.pending_peak_params = false;
            app.current_tab = Tab::Import;
            app.current_workflow = crate::app::Workflow::BioSym;
            app.status = "Import — file list / Ctrl+G generate".to_string();
            app.ensure_status_for_current_tab();
            return false;
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            app.pending_process = true;
            app.pending_peak_params = false;
            app.status = "Process: ↑↓ or 1–5   ←→/± window (MA/Median)   Enter apply   Esc cancel"
                .to_string();
            return false;
        }
        KeyCode::Char('k') => {
            // Peak detection on current series with current params
            app.status = crate::processing::run_peak_detection(app);
            return false;
        }
        KeyCode::Char('K') => {
            // Open peak-parameter editor (live re-detect on ←→; chart stays visible below)
            if app.loaded_signal.is_none() {
                app.status =
                    "No signal loaded — generate (Ctrl+G) or load a file first.".to_string();
                return false;
            }
            app.pending_process = false;
            app.pending_peak_params = true;
            // Seed a first detection so the waveform under the editor shows overlays.
            let msg = crate::processing::run_peak_detection(app);
            app.status = format!(
                "Peak params: ↑↓ field  ←→/± live  k re-run  Enter apply+close  d defaults  Esc — {}",
                msg
            );
            return false;
        }
        KeyCode::Char('t') => {
            if let Some(signal) = &mut app.loaded_signal {
                signal.show_known_peaks = !signal.show_known_peaks;
                app.status = format!(
                    "Known peaks overlay: {} ({} primary / {} secondary)",
                    if signal.show_known_peaks { "ON" } else { "OFF" },
                    signal.known_peaks_primary.len(),
                    signal.known_peaks_secondary.len()
                );
            }
            return false;
        }
        KeyCode::Char('T') => {
            if let Some(signal) = &mut app.loaded_signal {
                signal.show_detected_peaks = !signal.show_detected_peaks;
                app.status = format!(
                    "Detected peaks overlay: {} ({} peaks)",
                    if signal.show_detected_peaks {
                        "ON"
                    } else {
                        "OFF"
                    },
                    signal.detected_peaks.len()
                );
            }
            return false;
        }
        KeyCode::Char('i') => {
            // Toggle waveform ↔ tachogram (interval) view
            if app.loaded_signal.is_none() {
                app.status = "No signal loaded.".to_string();
                return false;
            }
            app.explore_view = app.explore_view.toggle();
            app.explore_scroll = 0;
            if app.explore_view == crate::app::ExploreView::Tachogram {
                // Ensure tachogram exists if we already have peaks
                if let Some(sig) = &mut app.loaded_signal {
                    if sig.tachogram.is_none() {
                        sig.rebuild_tachogram();
                    }
                }
                app.status = crate::processing::rebuild_tachogram_status(app);
                if app
                    .loaded_signal
                    .as_ref()
                    .and_then(|s| s.tachogram.as_ref())
                    .is_none()
                {
                    app.status = format!("Tachogram view — {}", app.status);
                } else {
                    app.status = format!("View: tachogram — {}", app.status);
                }
            } else {
                app.status = "View: waveform (peaks overlays as before)".to_string();
            }
            return false;
        }
        KeyCode::Char('o') => {
            // Cycle tachogram peak source (detected ↔ known primary)
            if let Some(sig) = &mut app.loaded_signal {
                sig.tachogram_source = sig.tachogram_source.toggle();
                app.explore_scroll = 0;
            }
            app.status = crate::processing::rebuild_tachogram_status(app);
            return false;
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            match crate::processing::export_tachogram(app) {
                Ok(path) => {
                    app.status = format!("Exported tachogram → {}", path.display());
                }
                Err(err) => {
                    app.status = format!("Tachogram export: {}", err);
                }
            }
            return false;
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if let Some(signal) = &mut app.loaded_signal {
                signal.reset();
                app.explore_scroll = 0;
                app.explore_view = crate::app::ExploreView::Waveform;
                app.status = "Reset to original (detected peaks + tachogram cleared).".to_string();
            }
            return false;
        }
        // Pan x-axis viewport for long BioSym signals / tachogram
        KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
            if app.loaded_signal.is_some() {
                let step = if app.explore_view == crate::app::ExploreView::Tachogram {
                    5usize
                } else {
                    30usize
                };
                app.explore_scroll = app.explore_scroll.saturating_sub(step);
                app.status = format!("Explore: pan x → start={}", app.explore_scroll);
            }
            return false;
        }
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
            if let Some(sig) = &app.loaded_signal {
                let (view_len, n, step) = if app.explore_view == crate::app::ExploreView::Tachogram
                {
                    let n = sig.tachogram.as_ref().map(|t| t.n_intervals()).unwrap_or(0);
                    (crate::ui::tabs::explore::TACHO_VIEW_LEN, n, 5usize)
                } else {
                    (
                        crate::ui::tabs::explore::EXPLORE_VIEW_LEN,
                        sig.current.len(),
                        30usize,
                    )
                };
                let max_start = n.saturating_sub(view_len);
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
            app.status = format!(
                "RQA params: m={} tau={} rad={:.2}  ←→/± rad  m/t  Enter=compute (RQA)  Esc",
                app.rqa_params.m, app.rqa_params.tau, app.rqa_params.radius
            );
            return false;
        }
        KeyCode::Char('x') | KeyCode::Char('X') => {
            // cRQA: prefer reference vs current; fallback to current vs time-reversed for demo
            if let Some(sig) = &app.loaded_signal {
                let (name_a, series_a, series_b) =
                    if let Some((ref_name, ref_data)) = &app.reference_series {
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
    use std::{
        fs::File,
        io::Write,
    };
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

fn calendar_status(app: &App) -> String {
    if app.daily_loads.is_empty() {
        return "Calendar empty — r: reload catalog  g: demo".to_string();
    }
    let idx = app
        .loadsym_scroll
        .min(app.daily_loads.len().saturating_sub(1));
    let date = app
        .daily_load_dates
        .get(idx)
        .cloned()
        .unwrap_or_else(|| format!("day {}", idx));
    let tss = app.daily_loads.get(idx).copied().unwrap_or(0.0);
    let src = if app.loadsym_from_catalog {
        "catalog"
    } else {
        "demo"
    };
    let rides = app.daily_ride_counts.get(idx).copied().unwrap_or(0);
    let widx = app.loadsym_week_scroll;
    format!(
        "Calendar [{}] {}  TSS={:.0} n={}  day {}/{} week {}/{}  ↑↓ day ←→ week  r:reload",
        src,
        date,
        tss,
        rides,
        idx + 1,
        app.daily_loads.len(),
        if app.weekly_loads.is_empty() {
            0
        } else {
            widx + 1
        },
        app.weekly_loads.len()
    )
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
                let _ = crate::processing::try_load_loadsym_catalog(app);
                crate::processing::focus_calendar_most_recent(app);
                app.status = calendar_status(app);
                return false;
            }
            KeyCode::Char('3') => {
                enter_loadsym_optimization(app);
                return false;
            }
            KeyCode::Enter => {
                match app.loadsym_selection {
                    0 => app.loadsym_view = crate::app::LoadSymView::Workout,
                    1 => {
                        app.loadsym_view = crate::app::LoadSymView::Calendar;
                        let _ = crate::processing::try_load_loadsym_catalog(app);
                        crate::processing::focus_calendar_most_recent(app);
                        app.status = calendar_status(app);
                    }
                    2 => enter_loadsym_optimization(app),
                    _ => {}
                }
                return false;
            }
            KeyCode::Char('g') | KeyCode::Char('G') => {
                crate::processing::apply_demo_daily_loads(app, 14);
                app.status =
                    "LoadSym: synthetic demo daily loads (r: reload real catalog)".to_string();
                return false;
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                match crate::processing::try_load_loadsym_catalog(app) {
                    Ok(true) => {
                        app.status = calendar_status(app);
                    }
                    Ok(false) => {
                        app.status =
                            "No catalog at $VELOFIT_HOME/db — run: symload db init && ingest"
                                .to_string();
                    }
                    Err(e) => {
                        app.status = format!("Catalog load error: {}", e);
                    }
                }
                return false;
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                if let Some(act) = crate::processing::find_newest_loadsym_activity(app) {
                    let n = act.times_s.len();
                    let src = act.source.clone();
                    app.loaded_activity = Some(act);
                    app.activity_scroll = 0;
                    app.activity_series = 0;
                    app.workout_user_thresh = 0.0;
                    app.workout_user_min_dur = 3;
                    app.loadsym_view = crate::app::LoadSymView::Workout;
                    app.status = format!(
                        "Loaded {} ({} samples). Roots: $VELOFIT_HOME + ./data",
                        src, n
                    );
                } else {
                    app.status =
                        "No .fit/.csv in $VELOFIT_HOME/raw|inbox or ./data/. Drop a file and press i."
                            .to_string();
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
                KeyCode::Char('i')
                | KeyCode::Char('I')
                | KeyCode::Char('a')
                | KeyCode::Char('A') => {
                    // Newest .fit under ~/velofit (raw/inbox) and project data dirs
                    if let Some(act) = crate::processing::find_newest_loadsym_activity(app) {
                        let n = act.times_s.len();
                        let src = act.source.clone();
                        app.loaded_activity = Some(act);
                        app.activity_scroll = 0;
                        app.activity_series = 0;
                        app.workout_user_thresh = 0.0;
                        app.workout_user_min_dur = 3;
                        app.status = format!(
                            "Loaded {} — {} samples. 1/2/3=series  ←→ scroll  f/F=FTP",
                            src, n
                        );
                    } else {
                        app.status = "No .fit/.csv in ~/velofit/raw|inbox or ./data|rides. Import via symload email fetch.".to_string();
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
                // Daily list (newest first on screen: ↓ = older, ↑ = newer)
                KeyCode::Up | KeyCode::Char('k') => {
                    app.loadsym_scroll =
                        (app.loadsym_scroll + 1).min(app.daily_loads.len().saturating_sub(1));
                    crate::processing::sync_week_scroll_from_daily(app);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if app.loadsym_scroll > 0 {
                        app.loadsym_scroll -= 1;
                    }
                    crate::processing::sync_week_scroll_from_daily(app);
                }
                // Weekly: ← older (past), → newer (future); list still newest-first
                KeyCode::Left | KeyCode::Char('h') => {
                    if app.loadsym_week_scroll > 0 {
                        app.loadsym_week_scroll -= 1;
                    }
                    if let Some(w) = app.weekly_loads.get(app.loadsym_week_scroll) {
                        app.loadsym_scroll = w.day_index_hi;
                        app.loadsym_scroll_from_week = true;
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if !app.weekly_loads.is_empty() {
                        app.loadsym_week_scroll = (app.loadsym_week_scroll + 1)
                            .min(app.weekly_loads.len().saturating_sub(1));
                    }
                    if let Some(w) = app.weekly_loads.get(app.loadsym_week_scroll) {
                        app.loadsym_scroll = w.day_index_hi;
                        app.loadsym_scroll_from_week = true;
                    }
                }
                KeyCode::Home => {
                    app.loadsym_scroll = 0;
                    crate::processing::sync_week_scroll_from_daily(app);
                }
                KeyCode::End => {
                    app.loadsym_scroll = app.daily_loads.len().saturating_sub(1);
                    crate::processing::sync_week_scroll_from_daily(app);
                }
                KeyCode::PageUp => {
                    app.loadsym_scroll = app.loadsym_scroll.saturating_sub(10);
                    crate::processing::sync_week_scroll_from_daily(app);
                }
                KeyCode::PageDown => {
                    app.loadsym_scroll =
                        (app.loadsym_scroll + 10).min(app.daily_loads.len().saturating_sub(1));
                    crate::processing::sync_week_scroll_from_daily(app);
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    match crate::processing::try_load_loadsym_catalog(app) {
                        Ok(true) => {
                            crate::processing::focus_calendar_most_recent(app);
                            app.status = calendar_status(app);
                        }
                        Ok(false) => {
                            app.status = "No catalog DB found — run symload ingest first".into()
                        }
                        Err(e) => app.status = format!("Catalog error: {}", e),
                    }
                    return false;
                }
                KeyCode::Char('g') | KeyCode::Char('G') => {
                    crate::processing::apply_demo_daily_loads(app, 14);
                    app.status = "Calendar: synthetic demo (r reloads catalog)".into();
                    return false;
                }
                KeyCode::Char('.') => {
                    // Jump to most recent day
                    crate::processing::focus_calendar_most_recent(app);
                    app.status = calendar_status(app);
                    return false;
                }
                KeyCode::Esc => {
                    app.loadsym_view = crate::app::LoadSymView::List;
                    app.status = "LoadSym — back to list".to_string();
                    return false;
                }
                _ => {}
            }
            app.status = calendar_status(app);
        }
        crate::app::LoadSymView::Optimization => match code {
            KeyCode::Esc => {
                app.loadsym_view = crate::app::LoadSymView::List;
                app.status = "LoadSym — back to list".to_string();
            }
            KeyCode::Char('1') => {
                app.loadsym_plan_goal = symworx_loadsym::load::LoadGoal::Recovery;
                crate::processing::ensure_loadsym_plan(app);
                app.status = opt_status(app);
            }
            KeyCode::Char('2') => {
                app.loadsym_plan_goal = symworx_loadsym::load::LoadGoal::Maintenance;
                crate::processing::ensure_loadsym_plan(app);
                app.status = opt_status(app);
            }
            KeyCode::Char('3') => {
                app.loadsym_plan_goal = symworx_loadsym::load::LoadGoal::Overload;
                crate::processing::ensure_loadsym_plan(app);
                app.status = opt_status(app);
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                if app.loadsym_plan_horizon > 2 {
                    app.loadsym_plan_horizon -= 1;
                    crate::processing::ensure_loadsym_plan(app);
                }
                app.status = opt_status(app);
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                let max_h = symworx_loadsym::load::MAX_HORIZON_DAYS;
                if app.loadsym_plan_horizon < max_h {
                    app.loadsym_plan_horizon += 1;
                    crate::processing::ensure_loadsym_plan(app);
                }
                app.status = opt_status(app);
            }
            // Enter: re-run plan with current goal/horizon (explicit recompute)
            KeyCode::Enter => {
                crate::processing::invalidate_loadsym_plan_cache(app);
                crate::processing::ensure_loadsym_plan(app);
                app.status = format!("Recomputed. {}", opt_status(app));
            }
            KeyCode::Char('g') | KeyCode::Char('G') => {
                crate::processing::apply_demo_daily_loads(app, 28);
                crate::processing::ensure_loadsym_plan(app);
                app.status = format!("Demo loads (28d). {}", opt_status(app));
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                match crate::processing::try_load_loadsym_catalog(app) {
                    Ok(true) => {
                        crate::processing::ensure_loadsym_plan(app);
                        app.status = format!("Catalog reloaded. {}", opt_status(app));
                    }
                    Ok(false) => {
                        app.status =
                            "No catalog — run symload db init && ingest, or g for demo".to_string();
                    }
                    Err(e) => app.status = format!("Catalog error: {}", e),
                }
            }
            _ => {}
        },
        _ => {}
    }
    false
}

fn enter_loadsym_optimization(app: &mut App) {
    app.loadsym_view = crate::app::LoadSymView::Optimization;
    if app.daily_loads.is_empty() {
        let _ = crate::processing::try_load_loadsym_catalog(app);
    }
    if app.daily_loads.is_empty() {
        app.status =
            "Optimization — no loads. r catalog / g demo · set H with −/+ · Enter recompute"
                .to_string();
    } else {
        crate::processing::ensure_loadsym_plan(app);
        app.status = opt_status(app);
    }
}

fn opt_status(app: &App) -> String {
    format!(
        "Plan goal={}  H={}d (max {})  · 1/2/3  −/+ days  Enter recompute  r/g  Esc",
        app.loadsym_plan_goal.as_str(),
        app.loadsym_plan_horizon,
        symworx_loadsym::load::MAX_HORIZON_DAYS,
    )
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
