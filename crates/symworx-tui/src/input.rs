use crossterm::event::{
    KeyCode,
    KeyModifiers,
};

use crate::app::{
    App,
    Tab,
};

pub fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    // Hard quit: Ctrl+Q always. Esc-Esc at root screens (see esc_root_or_quit).
    // Bare `q` is not quit — collides with typing.
    if matches!(code, KeyCode::Char('q') | KeyCode::Char('Q'))
        && modifiers.contains(KeyModifiers::CONTROL)
    {
        return true;
    }

    // Any non-Esc key disarms double-Esc quit.
    if code != KeyCode::Esc {
        app.clear_esc_quit();
    }

    if code == KeyCode::Char('?') && modifiers.contains(KeyModifiers::ALT) {
        app.help_mode = !app.help_mode;
        app.clear_esc_quit();
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
            app.clear_esc_quit();
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
            match app.current_workflow {
                crate::app::Workflow::StatsSym => {
                    app.stats_view = crate::app::StatsView::Import;
                    app.status =
                        "StatsSym Import — ↑↓ files  Enter load  / filter  Ctrl+G generate".into();
                }
                _ => {
                    app.current_tab = Tab::Import;
                    app.current_workflow = crate::app::Workflow::BioSym;
                    app.ensure_status_for_current_tab();
                }
            }
            return false;
        }
        KeyCode::Char('2') if modifiers.contains(KeyModifiers::CONTROL) => {
            match app.current_workflow {
                crate::app::Workflow::StatsSym => {
                    app.stats_view = crate::app::StatsView::Lab;
                    app.status = "StatsSym Lab — 1–4 task  x/y cols  Enter run  h residual".into();
                }
                _ => {
                    app.current_tab = Tab::Explore;
                    app.current_workflow = crate::app::Workflow::BioSym;
                    app.ensure_status_for_current_tab();
                }
            }
            return false;
        }
        KeyCode::Char('3') if modifiers.contains(KeyModifiers::CONTROL) => {
            match app.current_workflow {
                crate::app::Workflow::StatsSym => {
                    app.stats_view = crate::app::StatsView::Generate;
                    app.status =
                        "StatsSym Generate — ↑↓ preset  n/N size  s/S seed  +/− noise  Enter"
                            .into();
                }
                _ => {
                    app.current_tab = Tab::Dynamics;
                    app.current_workflow = crate::app::Workflow::BioSym;
                    app.ensure_status_for_current_tab();
                }
            }
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
            // Scope navigation to parent workflow's subtabs.
            match app.current_workflow {
                crate::app::Workflow::BioSym => {
                    app.pending_generate = false;
                    app.current_tab = if Tab::BIOSYM_TABS.contains(&app.current_tab) {
                        app.current_tab.biosym_prev()
                    } else {
                        Tab::Import
                    };
                    app.ensure_status_for_current_tab();
                }
                crate::app::Workflow::StatsSym => {
                    app.stats_view = app.stats_view.prev();
                    app.status = format!(
                        "StatsSym {}  ·  Ctrl+←→ tabs  ·  Ctrl+H Home",
                        app.stats_view.title()
                    );
                }
                crate::app::Workflow::LoadSym | crate::app::Workflow::SpatialSym => {
                    // Sub-views use in-workflow keys for now; footer still labels them.
                }
                crate::app::Workflow::Home => {}
            }
            return false;
        }
        KeyCode::Right if modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_workflow == crate::app::Workflow::Home {
                return false;
            }
            match app.current_workflow {
                crate::app::Workflow::BioSym => {
                    app.pending_generate = false;
                    app.current_tab = if Tab::BIOSYM_TABS.contains(&app.current_tab) {
                        app.current_tab.biosym_next()
                    } else {
                        Tab::Import
                    };
                    app.ensure_status_for_current_tab();
                }
                crate::app::Workflow::StatsSym => {
                    app.stats_view = app.stats_view.next();
                    app.status = format!(
                        "StatsSym {}  ·  Ctrl+←→ tabs  ·  Ctrl+H Home",
                        app.stats_view.title()
                    );
                }
                crate::app::Workflow::LoadSym | crate::app::Workflow::SpatialSym => {}
                crate::app::Workflow::Home => {}
            }
            return false;
        }
        KeyCode::Char('g') | KeyCode::Char('G') if modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_workflow == crate::app::Workflow::StatsSym {
                // StatsSym: open Generate tab (full preset UI — n/seed/noise).
                app.stats_view = crate::app::StatsView::Generate;
                app.manual_path.clear();
                app.file_filter.clear();
                app.filter_mode = false;
                app.status =
                    "StatsSym Generate — ↑↓ preset  n/N  s/S seed  +/− noise  Enter → Lab".into();
            } else {
                // BioSym: dedicated Generate tab (parity with StatsSym).
                app.current_workflow = crate::app::Workflow::BioSym;
                app.current_tab = Tab::Generate;
                app.pending_generate = false;
                app.manual_path.clear();
                app.file_filter.clear();
                app.filter_mode = false;
                app.status =
                    "BioSym Generate — ↑↓ preset  Enter generate → Explore  ·  1/2/3 quick".into();
            }
            return false;
        }
        // Live stream (simulator) — BioSym only. Bare `l`/`L` is Explore pan.
        KeyCode::Char('l') | KeyCode::Char('L') if modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_workflow == crate::app::Workflow::BioSym {
                app.start_live_simulator();
            } else {
                app.status =
                    "Ctrl+L live simulator is BioSym only — Home → 1 BioSym (or Explore), then Ctrl+L"
                        .to_string();
            }
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
        Tab::Generate => handle_bio_generate_keys(app, code),
        Tab::Spatial => handle_spatial_keys(app, code, modifiers),
        Tab::LoadSym => handle_loadsym_keys(app, code, modifiers),
        Tab::Stats => handle_stats_keys(app, code, modifiers),
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

fn run_bio_generate(app: &mut App, preset: crate::generate::DemoPreset) {
    app.pending_generate = false;
    app.manual_path.clear();
    if let Err(e) = crate::processing::generate_demo_and_load(app, preset) {
        app.status = format!("Generation failed: {e}");
    }
}

fn handle_bio_generate_keys(app: &mut App, code: KeyCode) -> bool {
    use crate::generate::DemoPreset;

    let n = DemoPreset::MENU.len();
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.bio_gen_preset > 0 {
                app.bio_gen_preset -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.bio_gen_preset + 1 < n {
                app.bio_gen_preset += 1;
            }
        }
        KeyCode::Char(c @ '1'..='3') => {
            let i = (c as u8 - b'1') as usize;
            if i < n {
                app.bio_gen_preset = i;
                run_bio_generate(app, DemoPreset::MENU[i]);
            }
        }
        KeyCode::Enter => {
            let i = app.bio_gen_preset.min(n - 1);
            run_bio_generate(app, DemoPreset::MENU[i]);
        }
        KeyCode::Esc => {
            app.pending_generate = false;
            app.current_tab = Tab::Import;
            app.clear_esc_quit();
            app.status = "Import — file list / Ctrl+G generate".to_string();
            app.ensure_status_for_current_tab();
        }
        _ => {}
    }
    false
}

fn handle_import_keys(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
    // Delete confirm modal — highest priority after generate overlay (Import only).
    if handle_pending_delete_keys(app, code) {
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
                app.clear_esc_quit();
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
        // Delete: only when not typing a manual path (so `x` can still be path text).
        KeyCode::Char('x') | KeyCode::Char('X') if app.manual_path.is_empty() => {
            arm_pending_delete(app);
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
                app.clear_esc_quit();
                return false;
            }
            // Import root (no path / filter / generate): Esc-Esc quits.
            return app.esc_root_or_quit();
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
    // Live stream mode: swallow most keys; Esc stops (before other Explore Esc).
    // Checked after peak/process? No — live is exclusive; check before those only if live
    // was started while they were closed. If user somehow had both, peak/process first.
    if app.is_live() && !app.pending_peak_params && !app.pending_process {
        match code {
            KeyCode::Esc => {
                app.stop_live_user();
                app.clear_esc_quit();
                return false;
            }
            // Allow pan-like keys to be no-ops with a hint; Ctrl+L restarts (handled globally).
            _ => {
                // Don't leak into offline Explore handlers while streaming.
                return false;
            }
        }
    }

    // Peak-parameter editor: checked before process and generic keys (input priority).
    if app.pending_peak_params {
        let n = crate::app::PeakDetectParams::N_FIELDS;
        match code {
            KeyCode::Esc => {
                app.pending_peak_params = false;
                app.clear_esc_quit();
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
                app.clear_esc_quit();
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
            // Live first (if peak/process already closed above)
            if app.is_live() {
                app.stop_live_user();
                app.clear_esc_quit();
                return false;
            }
            // Back to Import (BioSym file list / generate)
            app.pending_process = false;
            app.pending_peak_params = false;
            app.clear_esc_quit();
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
                app.clear_esc_quit();
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
        KeyCode::Esc => {
            app.clear_esc_quit();
            app.pending_rqa = false;
            app.current_tab = Tab::Explore;
            app.current_workflow = crate::app::Workflow::BioSym;
            app.status = "Explore — waveform / peaks / live".to_string();
            app.ensure_status_for_current_tab();
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
            if app.home_selection < 3 {
                app.home_selection += 1;
            }
            return false;
        }
        KeyCode::Char('1') => {
            app.switch_workflow(crate::app::Workflow::BioSym);
            return false;
        }
        KeyCode::Char('2') => {
            app.switch_workflow(crate::app::Workflow::StatsSym);
            return false;
        }
        KeyCode::Char('3') => {
            app.switch_workflow(crate::app::Workflow::LoadSym);
            return false;
        }
        KeyCode::Char('4') => {
            app.switch_workflow(crate::app::Workflow::SpatialSym);
            return false;
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if app.home_selection == 2 {
                app.status =
                    "LoadSym: 1 Workout  2 Metrics  3 Calendar  4 Optimization".to_string();
            }
            return false;
        }
        KeyCode::Char('i') | KeyCode::Char('I') => {
            if app.home_selection == 2 {
                app.status =
                    "LoadSym: import via activity files (FIT via symworx-io) — see LoadSym tab."
                        .to_string();
            } else if app.home_selection == 3 {
                app.switch_workflow(crate::app::Workflow::SpatialSym);
            }
            return false;
        }
        KeyCode::Enter => {
            match app.home_selection {
                0 => app.switch_workflow(crate::app::Workflow::BioSym),
                1 => app.switch_workflow(crate::app::Workflow::StatsSym),
                2 => app.switch_workflow(crate::app::Workflow::LoadSym),
                3 => app.switch_workflow(crate::app::Workflow::SpatialSym),
                _ => {}
            }
            return false;
        }
        KeyCode::Esc => {
            // Root: Esc-Esc quits (Ctrl+Q also works).
            return app.esc_root_or_quit();
        }
        _ => {}
    }
    false
}

/// Arm delete confirm for the selected Import file (`x`).
fn arm_pending_delete(app: &mut App) {
    match app.selected_path().cloned() {
        Some(path) if path.is_file() => {
            app.pending_delete = Some(path.clone());
            app.clear_esc_quit();
            app.status = format!("Delete {} ?   y confirm   n / Esc cancel", path.display());
        }
        Some(path) => {
            app.status = format!("Not a file: {}", path.display());
        }
        None => {
            app.status = "Select a file first, then x to delete".into();
        }
    }
}

/// Handle keys while `pending_delete` is armed. Returns true if the modal consumed the key.
fn handle_pending_delete_keys(app: &mut App, code: KeyCode) -> bool {
    if app.pending_delete.is_none() {
        return false;
    }
    match code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            confirm_pending_delete(app);
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.pending_delete = None;
            app.clear_esc_quit();
            app.status = "Delete cancelled".into();
        }
        // Swallow other keys so they don't leak into path/filter while confirming.
        _ => {}
    }
    true
}

fn confirm_pending_delete(app: &mut App) {
    let Some(path) = app.pending_delete.take() else {
        return;
    };
    let name = path.display().to_string();
    match std::fs::remove_file(&path) {
        Ok(()) => {
            // Best-effort: remove peaks sidecar next to biosignal CSVs.
            let peaks = crate::generate::peaks_sidecar_path(&path);
            if peaks.is_file() {
                let _ = std::fs::remove_file(&peaks);
            }
            app.refresh_file_list();
            app.ensure_valid_selection();
            app.status = format!("Deleted {name}");
        }
        Err(e) => {
            app.status = format!("Delete failed ({name}): {e}");
        }
    }
}

fn try_load_stats_table(app: &mut App, path: &str) {
    match symworx_io::load_numeric_table(path) {
        Ok(t) => {
            let skipped = if t.skipped_headers.is_empty() {
                String::new()
            } else {
                format!(" · skipped {}", t.skipped_headers.join(", "))
            };
            let src = format!("Imported {path}{skipped}");
            app.manual_path.clear();
            app.file_filter.clear();
            app.filter_mode = false;
            // Acquire → Lab workspace (same pattern as BioSym → Explore).
            app.enter_stats_lab_with_table(t, src, None);
        }
        Err(e) => app.status = format!("Load failed: {e}"),
    }
}

fn handle_stats_keys(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
    use crate::app::StatsView;

    match app.stats_view {
        StatsView::Import => {
            if handle_pending_delete_keys(app, code) {
                return false;
            }

            // Filter mode (same priority as BioSym Import).
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
                        app.clear_esc_quit();
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
                    app.status = "Filter files…".into();
                }
                KeyCode::Up => {
                    app.select_prev();
                }
                KeyCode::Down => {
                    app.select_next();
                }
                KeyCode::Char('x') | KeyCode::Char('X') if app.manual_path.is_empty() => {
                    arm_pending_delete(app);
                }
                KeyCode::Enter => {
                    // Prefer typed path; else selected file from discovery list.
                    let path = if !app.manual_path.trim().is_empty() {
                        Some(app.manual_path.trim().to_string())
                    } else {
                        app.selected_path().map(|p| p.display().to_string())
                    };
                    match path {
                        Some(p) => try_load_stats_table(app, &p),
                        None => app.status = "Select a .csv or type a path, then Enter".into(),
                    }
                }
                KeyCode::Char(c) if c.is_ascii() && !c.is_control() => {
                    app.manual_path.push(c);
                }
                KeyCode::Backspace => {
                    app.manual_path.pop();
                }
                KeyCode::Esc => {
                    if !app.manual_path.is_empty() {
                        app.manual_path.clear();
                        app.clear_esc_quit();
                        app.status = "Path cleared".into();
                    } else if !app.file_filter.is_empty() {
                        app.file_filter.clear();
                        app.ensure_valid_selection();
                        app.clear_esc_quit();
                        app.status = "Filter cleared".into();
                    } else {
                        // Import is StatsSym root (like BioSym Import).
                        return app.esc_root_or_quit();
                    }
                }
                _ => {}
            }
        }
        StatsView::Lab => match code {
            KeyCode::Esc => {
                // Lab → Import (BioSym Explore → Import).
                app.stats_view = StatsView::Import;
                app.clear_esc_quit();
                app.status = "StatsSym Import — ↑↓ files  Enter load  Ctrl+G generate".into();
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                let n = crate::app::StatsLabTask::ALL.len();
                app.stats_lab_task = (app.stats_lab_task + 1) % n;
                app.status = format!(
                    "Lab task: {}",
                    crate::app::StatsLabTask::ALL[app.stats_lab_task].label()
                );
            }
            KeyCode::Char(c @ '1'..='6') => {
                let i = (c as u8 - b'1') as usize;
                if i < crate::app::StatsLabTask::ALL.len() {
                    app.stats_lab_task = i;
                    app.status = format!("Lab task: {}", crate::app::StatsLabTask::ALL[i].label());
                }
            }
            // Poly: max degree (Enter re-runs).
            KeyCode::Char('d') => {
                app.stats_poly_max_degree = (app.stats_poly_max_degree + 1).min(8);
                app.status = format!(
                    "Poly max degree = {}  ·  Enter to re-run",
                    app.stats_poly_max_degree
                );
            }
            KeyCode::Char('D') => {
                app.stats_poly_max_degree = app.stats_poly_max_degree.saturating_sub(1).max(1);
                app.status = format!(
                    "Poly max degree = {}  ·  Enter to re-run",
                    app.stats_poly_max_degree
                );
            }
            KeyCode::Char('x') => {
                if let Some(ref t) = app.stats_table {
                    app.stats_lab_x_col = (app.stats_lab_x_col + 1) % t.n_cols().max(1);
                    let name = crate::app::App::stats_col_name(t, app.stats_lab_x_col);
                    app.status = format!("X = {} [{}]", name, app.stats_lab_x_col);
                }
            }
            KeyCode::Char('X') => {
                if let Some(ref t) = app.stats_table {
                    let n = t.n_cols().max(1);
                    app.stats_lab_x_col = (app.stats_lab_x_col + n - 1) % n;
                    let name = crate::app::App::stats_col_name(t, app.stats_lab_x_col);
                    app.status = format!("X = {} [{}]", name, app.stats_lab_x_col);
                }
            }
            KeyCode::Char('y') => {
                if let Some(ref t) = app.stats_table {
                    app.stats_lab_y_col = (app.stats_lab_y_col + 1) % t.n_cols().max(1);
                    let name = crate::app::App::stats_col_name(t, app.stats_lab_y_col);
                    app.status = format!("Y = {} [{}]", name, app.stats_lab_y_col);
                }
            }
            KeyCode::Char('Y') => {
                if let Some(ref t) = app.stats_table {
                    let n = t.n_cols().max(1);
                    app.stats_lab_y_col = (app.stats_lab_y_col + n - 1) % n;
                    let name = crate::app::App::stats_col_name(t, app.stats_lab_y_col);
                    app.status = format!("Y = {} [{}]", name, app.stats_lab_y_col);
                }
            }
            KeyCode::Char('h') | KeyCode::Char('H') => {
                app.stats_residual_mode = match app.stats_residual_mode {
                    crate::app::ResidualPanelMode::BlandAltman => {
                        crate::app::ResidualPanelMode::Histogram
                    }
                    crate::app::ResidualPanelMode::Histogram => {
                        crate::app::ResidualPanelMode::BlandAltman
                    }
                };
                app.status = match app.stats_residual_mode {
                    crate::app::ResidualPanelMode::BlandAltman => {
                        "Residual panel: Bland–Altman".into()
                    }
                    crate::app::ResidualPanelMode::Histogram => "Residual panel: histogram".into(),
                };
            }
            // Pipeline: model (OLS / Logistic) and fold count.
            KeyCode::Char('m') | KeyCode::Char('M') => {
                app.stats_pipeline_model = app.stats_pipeline_model.next();
                app.status = format!(
                    "Pipeline model = {}  ·  Enter to re-run",
                    app.stats_pipeline_model.label()
                );
            }
            KeyCode::Char('k') => {
                app.stats_pipeline_k = (app.stats_pipeline_k + 1).min(10);
                app.status = format!(
                    "Pipeline folds k={}  ·  Enter to re-run",
                    app.stats_pipeline_k
                );
            }
            KeyCode::Char('K') => {
                app.stats_pipeline_k = app.stats_pipeline_k.saturating_sub(1).max(2);
                app.status = format!(
                    "Pipeline folds k={}  ·  Enter to re-run",
                    app.stats_pipeline_k
                );
            }
            // Focus metrics table row (Pipeline splits / Poly degrees) — plots follow.
            KeyCode::Char('f') | KeyCode::Char('F') | KeyCode::Down => {
                if let Some(ref mut res) = app.stats_lab_result {
                    if !res.metrics_rows.is_empty() {
                        res.focus_next();
                        let lab = res
                            .metrics_rows
                            .get(res.focused_row)
                            .map(|r| r.label.as_str())
                            .unwrap_or("?");
                        let kind = if res.task == crate::app::StatsLabTask::Poly {
                            "Poly"
                        } else {
                            "Pipeline"
                        };
                        app.status = format!("{kind} focus: {lab}");
                    }
                }
            }
            KeyCode::Up => {
                if let Some(ref mut res) = app.stats_lab_result {
                    if !res.metrics_rows.is_empty() {
                        res.focus_prev();
                        let lab = res
                            .metrics_rows
                            .get(res.focused_row)
                            .map(|r| r.label.as_str())
                            .unwrap_or("?");
                        let kind = if res.task == crate::app::StatsLabTask::Poly {
                            "Poly"
                        } else {
                            "Pipeline"
                        };
                        app.status = format!("{kind} focus: {lab}");
                    }
                }
            }
            KeyCode::Enter => {
                if let Some(ref table) = app.stats_table {
                    let task = crate::app::StatsLabTask::ALL[app
                        .stats_lab_task
                        .min(crate::app::StatsLabTask::ALL.len() - 1)];
                    match crate::stats_lab::run_lab(
                        table,
                        task,
                        app.stats_lab_x_col,
                        app.stats_lab_y_col,
                        crate::stats_lab::LabRunOpts {
                            pipeline_k: app.stats_pipeline_k,
                            poly_max_degree: app.stats_poly_max_degree,
                            pipeline_model: app.stats_pipeline_model,
                        },
                    ) {
                        Ok(res) => {
                            let extra = if !res.metrics_rows.is_empty() {
                                let lab = res
                                    .metrics_rows
                                    .get(res.focused_row)
                                    .map(|r| r.label.as_str())
                                    .unwrap_or("?");
                                format!(" · focus {lab}  ·  {} splits", res.metrics_rows.len())
                            } else {
                                String::new()
                            };
                            app.status = format!("Lab: {} done{extra}", task.label());
                            app.stats_lab_result = Some(res);
                        }
                        Err(e) => app.status = format!("Lab error: {e}"),
                    }
                } else {
                    app.status = "Load or generate a table first".into();
                }
            }
            _ => {}
        },
        StatsView::Generate => match code {
            KeyCode::Esc => {
                // Generate → Import (cancel / back).
                app.stats_view = StatsView::Import;
                app.clear_esc_quit();
                app.status = "StatsSym Import — ↑↓ files  Enter load  Ctrl+G generate".into();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if app.stats_gen_preset > 0 {
                    app.stats_gen_preset -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = symworx_stats::SyntheticPreset::ALL.len().saturating_sub(1);
                if app.stats_gen_preset < max {
                    app.stats_gen_preset += 1;
                }
            }
            KeyCode::Char(c @ '1'..='6') => {
                let i = (c as u8 - b'1') as usize;
                if i < symworx_stats::SyntheticPreset::ALL.len() {
                    app.stats_gen_preset = i;
                }
            }
            KeyCode::Char('n') => {
                app.stats_gen_n = (app.stats_gen_n + 50).min(5000);
                app.status = format!("n = {}", app.stats_gen_n);
            }
            KeyCode::Char('N') => {
                app.stats_gen_n = app.stats_gen_n.saturating_sub(50).max(20);
                app.status = format!("n = {}", app.stats_gen_n);
            }
            KeyCode::Char('s') => {
                app.stats_gen_seed = app.stats_gen_seed.saturating_add(1);
                app.status = format!("seed = {}", app.stats_gen_seed);
            }
            KeyCode::Char('S') => {
                app.stats_gen_seed = app.stats_gen_seed.saturating_sub(1);
                app.status = format!("seed = {}", app.stats_gen_seed);
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                app.stats_gen_noise = (app.stats_gen_noise + 0.1).min(5.0);
                app.status = format!("noise = {:.2}", app.stats_gen_noise);
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                app.stats_gen_noise = (app.stats_gen_noise - 0.1).max(0.05);
                app.status = format!("noise = {:.2}", app.stats_gen_noise);
            }
            KeyCode::Enter => {
                run_stats_generate(app);
            }
            _ => {}
        },
    }
    false
}

fn run_stats_generate(app: &mut App) {
    use symworx_stats::{
        generate_synthetic,
        SyntheticPreset,
        SyntheticSpec,
    };

    use crate::app::StatsLabTask;

    let presets = SyntheticPreset::ALL;
    let preset = presets[app.stats_gen_preset.min(presets.len() - 1)];
    let spec = SyntheticSpec {
        n: app.stats_gen_n.max(20),
        seed: app.stats_gen_seed,
        noise: app.stats_gen_noise,
        ..Default::default()
    };

    // Teaching presets → sensible Lab task (no auto-run).
    let task_hint = match preset {
        SyntheticPreset::LinearRegression => Some(StatsLabTask::Regress),
        SyntheticPreset::BivariateCorrelated => Some(StatsLabTask::Correlate),
        SyntheticPreset::Normal1D => Some(StatsLabTask::Describe),
        SyntheticPreset::TwoClassBlobs | SyntheticPreset::ThreeClassBlobs => {
            // Classify first; seed Pipeline model so m/Enter path is ready.
            app.stats_pipeline_model = crate::app::PipelineModel::Logistic;
            Some(StatsLabTask::Classify)
        }
        SyntheticPreset::Cluster3 => Some(StatsLabTask::Describe),
    };

    match generate_synthetic(preset, &spec) {
        Ok(synth) => {
            let _ = std::fs::create_dir_all("data");
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let path = format!(
                "data/stats_synth_{}_{}.csv",
                preset.label().replace(' ', "_").to_lowercase(),
                ts
            );
            if let Err(e) = symworx_io::write_columns_csv(&path, &synth.headers, &synth.columns) {
                app.status = format!("Write failed: {e}");
                return;
            }
            match symworx_io::load_numeric_table(&path) {
                Ok(t) => {
                    app.stats_gen_notes = synth.notes.clone();
                    let src = format!("Generated {} → {path}", preset.label());
                    // Acquire → Lab workspace (BioSym Generate → Explore analogue).
                    app.enter_stats_lab_with_table(t, src, task_hint);
                }
                Err(e) => {
                    app.stats_gen_notes = synth.notes;
                    app.status = format!("Wrote {path} but reload failed: {e}");
                }
            }
        }
        Err(e) => app.status = format!("Generate failed: {e}"),
    }
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
    let day_rides = crate::processing::rides_for_focus_day(app);
    let n_files = day_rides.len();
    let ride_i = if n_files == 0 {
        0
    } else {
        app.calendar_ride_sel.min(n_files - 1) + 1
    };
    let widx = app.loadsym_week_scroll;
    format!(
        "[{}] {} TSLi={:.0}  file {}/{}  day {}/{} week {}/{}  · n/p ride  Enter/o open  r reload",
        src,
        date,
        tss,
        ride_i,
        n_files,
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
    // Workout file-open modal swallows keys (same priority idea as Import modals).
    if app.pending_workout_open {
        return handle_workout_open_modal(app, code);
    }

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
                if app.loadsym_selection < 3 {
                    app.loadsym_selection += 1;
                }
                return false;
            }
            KeyCode::Char('1') => {
                app.loadsym_view = crate::app::LoadSymView::Workout;
                app.status =
                    "Workout: o open file  i newest  1–5 streams  ←→ pan  Esc list".to_string();
                return false;
            }
            KeyCode::Char('2') => {
                enter_loadsym_metrics(app);
                return false;
            }
            KeyCode::Char('3') => {
                app.loadsym_view = crate::app::LoadSymView::Calendar;
                let _ = crate::processing::try_load_loadsym_catalog(app);
                crate::processing::focus_calendar_most_recent(app);
                crate::processing::clamp_calendar_ride_sel(app);
                app.status = calendar_status(app);
                return false;
            }
            KeyCode::Char('4') => {
                enter_loadsym_optimization(app);
                return false;
            }
            KeyCode::Enter => {
                match app.loadsym_selection {
                    0 => {
                        app.loadsym_view = crate::app::LoadSymView::Workout;
                        app.status =
                            "Workout: o open file  i newest  1–5 streams  Esc list".to_string();
                    }
                    1 => enter_loadsym_metrics(app),
                    2 => {
                        app.loadsym_view = crate::app::LoadSymView::Calendar;
                        let _ = crate::processing::try_load_loadsym_catalog(app);
                        crate::processing::focus_calendar_most_recent(app);
                        crate::processing::clamp_calendar_ride_sel(app);
                        app.status = calendar_status(app);
                    }
                    3 => enter_loadsym_optimization(app),
                    _ => {}
                }
                return false;
            }
            KeyCode::Char('g') | KeyCode::Char('G') => {
                crate::processing::apply_demo_daily_loads(app, 14);
                app.loadsym_goal_user_override = false;
                app.status =
                    "LoadSym: synthetic demo daily loads (r: reload real catalog)".to_string();
                return false;
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                match crate::processing::try_load_loadsym_catalog(app) {
                    Ok(true) => {
                        app.loadsym_goal_user_override = false;
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
                    // Reuse shared loader path
                    let path = std::path::PathBuf::from(&act.source);
                    if let Ok(msg) = crate::processing::load_activity_into_app(app, &path) {
                        app.loadsym_view = crate::app::LoadSymView::Workout;
                        app.status = format!("{msg}. Roots: $VELOFIT_HOME + ./data");
                    } else {
                        // Activity already parsed — install directly
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
                    }
                } else {
                    app.status =
                        "No .fit/.csv in $VELOFIT_HOME/raw|inbox or ./data/. Drop a file and press i or o."
                            .to_string();
                }
                return false;
            }
            KeyCode::Char('o') | KeyCode::Char('O') => {
                crate::processing::refresh_workout_file_list(app);
                app.pending_workout_open = true;
                app.loadsym_view = crate::app::LoadSymView::Workout;
                app.status = if app.workout_file_list.is_empty() {
                    "No activity files found — check $VELOFIT_HOME/raw|inbox".to_string()
                } else {
                    format!(
                        "Open file: {} candidates  ↑↓ select  Enter load  Esc cancel",
                        app.workout_file_list.len()
                    )
                };
                return false;
            }
            KeyCode::Esc => {
                // LoadSym home list is a root: Esc-Esc quits.
                return app.esc_root_or_quit();
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
                KeyCode::Char('o') | KeyCode::Char('O') => {
                    crate::processing::refresh_workout_file_list(app);
                    app.pending_workout_open = true;
                    app.status = if app.workout_file_list.is_empty() {
                        "No activity files found — check $VELOFIT_HOME/raw|inbox".to_string()
                    } else {
                        format!(
                            "Open file: {} candidates  ↑↓ select  Enter load  Esc cancel",
                            app.workout_file_list.len()
                        )
                    };
                }
                KeyCode::Char('i')
                | KeyCode::Char('I')
                | KeyCode::Char('a')
                | KeyCode::Char('A') => {
                    // Newest .fit under ~/velofit (raw/inbox) and project data dirs
                    if let Some(act) = crate::processing::find_newest_loadsym_activity(app) {
                        let path = std::path::PathBuf::from(&act.source);
                        if let Ok(msg) = crate::processing::load_activity_into_app(app, &path) {
                            app.status = format!("{msg}. 1/2/3=series  ←→ scroll  o=open  f/F=FTP");
                        } else {
                            let n = act.times_s.len();
                            let src = act.source.clone();
                            app.loaded_activity = Some(act);
                            app.activity_scroll = 0;
                            app.activity_series = 0;
                            app.workout_user_thresh = 0.0;
                            app.workout_user_min_dur = 3;
                            app.status = format!(
                                "Loaded {} — {} samples. 1/2/3=series  ←→ scroll  o=open  f/F=FTP",
                                src, n
                            );
                        }
                    } else {
                        app.status = "No .fit/.csv in ~/velofit/raw|inbox or ./data. Press o to browse, or import via symload.".to_string();
                    }
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    app.loaded_activity = None;
                    app.activity_scroll = 0;
                    app.activity_series = 0;
                    app.workout_stream_on = [true, true, true, false, false];
                    app.workout_user_thresh = 0.0;
                    app.workout_user_min_dur = 3;
                    app.status =
                        "Cleared activity. Press o/i to load a file (no demo series).".to_string();
                }
                KeyCode::Char('1') => {
                    app.status = crate::processing::toggle_workout_panel(app, 0);
                }
                KeyCode::Char('2') => {
                    app.status = crate::processing::toggle_workout_panel(app, 1);
                }
                KeyCode::Char('3') => {
                    app.status = crate::processing::toggle_workout_panel(app, 2);
                }
                KeyCode::Char('4') => {
                    app.status = crate::processing::toggle_workout_panel(app, 3);
                }
                KeyCode::Char('5') => {
                    app.status = crate::processing::toggle_workout_panel(app, 4);
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
                    app.clear_esc_quit();
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
                    app.calendar_ride_sel = 0;
                    crate::processing::sync_week_scroll_from_daily(app);
                    crate::processing::clamp_calendar_ride_sel(app);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if app.loadsym_scroll > 0 {
                        app.loadsym_scroll -= 1;
                    }
                    app.calendar_ride_sel = 0;
                    crate::processing::sync_week_scroll_from_daily(app);
                    crate::processing::clamp_calendar_ride_sel(app);
                }
                // Weekly: ← older (past), → newer (future); list still newest-first
                KeyCode::Left | KeyCode::Char('h') => {
                    if app.loadsym_week_scroll > 0 {
                        app.loadsym_week_scroll -= 1;
                    }
                    if let Some(w) = app.weekly_loads.get(app.loadsym_week_scroll) {
                        app.loadsym_scroll = w.day_index_hi;
                        app.loadsym_scroll_from_week = true;
                        app.calendar_ride_sel = 0;
                    }
                    crate::processing::clamp_calendar_ride_sel(app);
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if !app.weekly_loads.is_empty() {
                        app.loadsym_week_scroll = (app.loadsym_week_scroll + 1)
                            .min(app.weekly_loads.len().saturating_sub(1));
                    }
                    if let Some(w) = app.weekly_loads.get(app.loadsym_week_scroll) {
                        app.loadsym_scroll = w.day_index_hi;
                        app.loadsym_scroll_from_week = true;
                        app.calendar_ride_sel = 0;
                    }
                    crate::processing::clamp_calendar_ride_sel(app);
                }
                // Ride sub-selection on focused day
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    let n = crate::processing::rides_for_focus_day(app).len();
                    if n > 0 {
                        app.calendar_ride_sel = (app.calendar_ride_sel + 1) % n;
                    }
                    crate::processing::clamp_calendar_ride_sel(app);
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    let n = crate::processing::rides_for_focus_day(app).len();
                    if n > 0 {
                        app.calendar_ride_sel = if app.calendar_ride_sel == 0 {
                            n - 1
                        } else {
                            app.calendar_ride_sel - 1
                        };
                    }
                    crate::processing::clamp_calendar_ride_sel(app);
                }
                KeyCode::Enter | KeyCode::Char('o') | KeyCode::Char('O') => {
                    let _ = crate::processing::open_calendar_ride_into_workout(app);
                    return false;
                }
                KeyCode::Home => {
                    app.loadsym_scroll = 0;
                    app.calendar_ride_sel = 0;
                    crate::processing::sync_week_scroll_from_daily(app);
                }
                KeyCode::End => {
                    app.loadsym_scroll = app.daily_loads.len().saturating_sub(1);
                    app.calendar_ride_sel = 0;
                    crate::processing::sync_week_scroll_from_daily(app);
                }
                KeyCode::PageUp => {
                    app.loadsym_scroll = app.loadsym_scroll.saturating_sub(10);
                    app.calendar_ride_sel = 0;
                    crate::processing::sync_week_scroll_from_daily(app);
                }
                KeyCode::PageDown => {
                    app.loadsym_scroll =
                        (app.loadsym_scroll + 10).min(app.daily_loads.len().saturating_sub(1));
                    app.calendar_ride_sel = 0;
                    crate::processing::sync_week_scroll_from_daily(app);
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    match crate::processing::try_load_loadsym_catalog(app) {
                        Ok(true) => {
                            crate::processing::focus_calendar_most_recent(app);
                            app.calendar_ride_sel = 0;
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
                    app.calendar_ride_sel = 0;
                    app.status = "Calendar: synthetic demo (r reloads catalog)".into();
                    return false;
                }
                KeyCode::Char('.') => {
                    // Jump to most recent day
                    crate::processing::focus_calendar_most_recent(app);
                    app.calendar_ride_sel = 0;
                    app.status = calendar_status(app);
                    return false;
                }
                KeyCode::Esc => {
                    app.loadsym_view = crate::app::LoadSymView::List;
                    app.clear_esc_quit();
                    app.status = "LoadSym — back to list".to_string();
                    return false;
                }
                _ => {}
            }
            app.status = calendar_status(app);
        }
        crate::app::LoadSymView::Metrics => match code {
            KeyCode::Esc => {
                app.loadsym_view = crate::app::LoadSymView::List;
                app.clear_esc_quit();
                app.status = "LoadSym — back to list".to_string();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                // Newest-first UI: ↑ = newer (higher storage index)
                if !app.catalog_activity_metrics.is_empty() {
                    app.metrics_scroll = (app.metrics_scroll + 1)
                        .min(app.catalog_activity_metrics.len().saturating_sub(1));
                }
                app.status = metrics_status(app);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.metrics_scroll > 0 {
                    app.metrics_scroll -= 1;
                }
                app.status = metrics_status(app);
            }
            KeyCode::Home => {
                app.metrics_scroll = 0;
                app.status = metrics_status(app);
            }
            KeyCode::End => {
                app.metrics_scroll = app.catalog_activity_metrics.len().saturating_sub(1);
                app.status = metrics_status(app);
            }
            KeyCode::PageUp => {
                app.metrics_scroll = (app.metrics_scroll + 10)
                    .min(app.catalog_activity_metrics.len().saturating_sub(1));
                app.status = metrics_status(app);
            }
            KeyCode::PageDown => {
                app.metrics_scroll = app.metrics_scroll.saturating_sub(10);
                app.status = metrics_status(app);
            }
            KeyCode::Enter | KeyCode::Char('o') | KeyCode::Char('O') => {
                let _ = crate::processing::open_metrics_row_into_workout(app);
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                match crate::processing::try_load_loadsym_catalog(app) {
                    Ok(true) => app.status = metrics_status(app),
                    Ok(false) => app.status = "No catalog — run symload db init && ingest".into(),
                    Err(e) => app.status = format!("Catalog error: {e}"),
                }
            }
            // v: toggle trend ↔ bi-plot
            KeyCode::Char('v') | KeyCode::Char('V') => {
                use crate::app::MetricsChartMode;
                app.metrics_chart_mode = match app.metrics_chart_mode {
                    MetricsChartMode::Trend => MetricsChartMode::Biplot,
                    MetricsChartMode::Biplot => MetricsChartMode::Trend,
                };
                app.status = metrics_status(app);
            }
            // 1–8: set trend Y, or bi-plot Y
            KeyCode::Char(c @ '1'..='8') => {
                if let Some(f) = crate::app::MetricsField::from_digit(c) {
                    match app.metrics_chart_mode {
                        crate::app::MetricsChartMode::Trend => {
                            app.metrics_trend_field = f;
                        }
                        crate::app::MetricsChartMode::Biplot => {
                            app.metrics_biplot_y = f;
                        }
                    }
                    app.status = metrics_status(app);
                }
            }
            KeyCode::Char('x') => {
                app.metrics_biplot_x = app.metrics_biplot_x.next();
                app.metrics_chart_mode = crate::app::MetricsChartMode::Biplot;
                app.status = metrics_status(app);
            }
            KeyCode::Char('X') => {
                // reverse cycle: next seven times = previous in 8-element ring
                for _ in 0..7 {
                    app.metrics_biplot_x = app.metrics_biplot_x.next();
                }
                app.metrics_chart_mode = crate::app::MetricsChartMode::Biplot;
                app.status = metrics_status(app);
            }
            KeyCode::Char('y') => {
                app.metrics_biplot_y = app.metrics_biplot_y.next();
                app.metrics_chart_mode = crate::app::MetricsChartMode::Biplot;
                app.status = metrics_status(app);
            }
            KeyCode::Char('Y') => {
                for _ in 0..7 {
                    app.metrics_biplot_y = app.metrics_biplot_y.next();
                }
                app.metrics_chart_mode = crate::app::MetricsChartMode::Biplot;
                app.status = metrics_status(app);
            }
            _ => {}
        },
        crate::app::LoadSymView::Optimization => match code {
            KeyCode::Esc => {
                app.loadsym_view = crate::app::LoadSymView::List;
                app.clear_esc_quit();
                app.status = "LoadSym — back to list".to_string();
            }
            KeyCode::Char('1') => {
                app.loadsym_plan_goal = symworx_loadsym::load::LoadGoal::Recovery;
                app.loadsym_goal_user_override = true;
                crate::processing::ensure_loadsym_plan(app);
                app.status = opt_status(app);
            }
            KeyCode::Char('2') => {
                app.loadsym_plan_goal = symworx_loadsym::load::LoadGoal::Maintenance;
                app.loadsym_goal_user_override = true;
                crate::processing::ensure_loadsym_plan(app);
                app.status = opt_status(app);
            }
            KeyCode::Char('3') => {
                app.loadsym_plan_goal = symworx_loadsym::load::LoadGoal::Overload;
                app.loadsym_goal_user_override = true;
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
                app.loadsym_goal_user_override = false;
                crate::processing::apply_suggested_load_goal(app, true);
                crate::processing::ensure_loadsym_plan(app);
                app.status = format!("Demo loads (28d). {}", opt_status(app));
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                match crate::processing::try_load_loadsym_catalog(app) {
                    Ok(true) => {
                        // Re-suggest only if user has not overridden goal.
                        crate::processing::apply_suggested_load_goal(app, false);
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

/// Handle keys while the Workout "open file" modal is active.
fn handle_workout_open_modal(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Esc => {
            app.pending_workout_open = false;
            app.clear_esc_quit();
            app.status = "Open cancelled".to_string();
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
            if app.workout_file_sel > 0 {
                app.workout_file_sel -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
            if !app.workout_file_list.is_empty() {
                app.workout_file_sel =
                    (app.workout_file_sel + 1).min(app.workout_file_list.len().saturating_sub(1));
            }
        }
        KeyCode::Home => {
            app.workout_file_sel = 0;
        }
        KeyCode::End => {
            app.workout_file_sel = app.workout_file_list.len().saturating_sub(1);
        }
        KeyCode::PageUp => {
            app.workout_file_sel = app.workout_file_sel.saturating_sub(10);
        }
        KeyCode::PageDown => {
            if !app.workout_file_list.is_empty() {
                app.workout_file_sel =
                    (app.workout_file_sel + 10).min(app.workout_file_list.len().saturating_sub(1));
            }
        }
        KeyCode::Enter => {
            let _ = crate::processing::open_selected_workout_file(app);
        }
        // Swallow all other keys so they do not leak into parent handlers.
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
        app.loadsym_goal_suggest_note.clear();
        app.status =
            "Optimization — no loads. r catalog / g demo · set H with −/+ · Enter recompute"
                .to_string();
    } else {
        // Fresh enter: re-suggest goal from form/fatigue/ACLi (user can override with 1/2/3).
        crate::processing::apply_suggested_load_goal(app, true);
        crate::processing::ensure_loadsym_plan(app);
        app.status = opt_status(app);
    }
}

fn enter_loadsym_metrics(app: &mut App) {
    app.loadsym_view = crate::app::LoadSymView::Metrics;
    if app.catalog_activity_metrics.is_empty() {
        let _ = crate::processing::try_load_loadsym_catalog(app);
    }
    if app.catalog_activity_metrics.is_empty() {
        app.status =
            "Metrics — empty. r catalog after symload ingest · Enter opens ride in Workout"
                .to_string();
    } else {
        app.metrics_scroll = app.catalog_activity_metrics.len().saturating_sub(1);
        app.status = metrics_status(app);
    }
}

fn metrics_status(app: &App) -> String {
    let n = app.catalog_activity_metrics.len();
    if n == 0 {
        return "Metrics empty — r reload catalog".into();
    }
    let i = app.metrics_scroll.min(n - 1);
    let r = &app.catalog_activity_metrics[i];
    let name = r.source_file.rsplit('/').next().unwrap_or(&r.source_file);
    let chart = match app.metrics_chart_mode {
        crate::app::MetricsChartMode::Trend => {
            format!("trend Y={}", app.metrics_trend_field.label())
        }
        crate::app::MetricsChartMode::Biplot => format!(
            "biplot {} vs {}",
            app.metrics_biplot_y.label(),
            app.metrics_biplot_x.label()
        ),
    };
    format!(
        "Metrics {}/{}  {}  {}  TSLi={}  ·  {}  ·  v toggle  Enter open",
        i + 1,
        n,
        r.ride_date,
        name,
        r.tss
            .map(|v| format!("{:.0}", v))
            .unwrap_or_else(|| "-".into()),
        chart,
    )
}

fn opt_status(app: &App) -> String {
    let note = if app.loadsym_goal_suggest_note.is_empty() {
        String::new()
    } else if app.loadsym_goal_user_override {
        format!("  · override · was: {}", app.loadsym_goal_suggest_note)
    } else {
        format!("  · {}", app.loadsym_goal_suggest_note)
    };
    format!(
        "Plan goal={}  H={}d (max {}){}  · 1/2/3  −/+  Enter  r/g  Esc",
        app.loadsym_plan_goal.as_str(),
        app.loadsym_plan_horizon,
        symworx_loadsym::load::MAX_HORIZON_DAYS,
        note,
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
