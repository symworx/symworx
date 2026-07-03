use crate::app::{App, Tab};
use crossterm::event::{KeyCode, KeyModifiers};


pub fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    if code == KeyCode::Char('q') {
        return true;
    }

    if code == KeyCode::Char('?') && modifiers.contains(KeyModifiers::ALT) {
        app.help_mode = !app.help_mode;
        return false;
    }

    // Refresh must be reliable (even in submodes / while typing) — early return per conventions
    if (code == KeyCode::Char('r') && modifiers.contains(KeyModifiers::CONTROL)) || code == KeyCode::F(5) {
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
        if (code == KeyCode::Char('h') || code == KeyCode::Char('H')) && modifiers.contains(KeyModifiers::CONTROL) {
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
    if (code == KeyCode::Char('h') || code == KeyCode::Char('H')) && modifiers.contains(KeyModifiers::CONTROL) {
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
            if app.current_workflow == crate::app::Workflow::Home { return false; }
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
            if app.current_workflow == crate::app::Workflow::Home { return false; }
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
            app.status = "BioSym demo data: 1 = Resting PPG   2 = Respiration   3 = Stride   Esc = cancel".to_string();
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
        Tab::LoadSym => false, // no special keys on empty template yet
        Tab::Home => handle_home_keys(app, code, modifiers),
    }
}

fn handle_spatial_keys(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
    // Sub-view switching and import/generate menu (sub-tab equivalent) before viz nav
    if app.pending_spatial_import {
        match code {
            KeyCode::Char('1') => {
                // synthetic regen as option 1
                app.seed_spatial_demo();
                app.pending_spatial_import = false;
                app.spatial_view = crate::app::SpatialView::Visualize;
                app.status = "Spatial: generated synthetic (preset)".to_string();
                return false;
            }
            KeyCode::Char('2') | KeyCode::Char('i') | KeyCode::Char('I') => {
                // Placeholder "import" a synthetic match
                app.status = "Spatial: loaded placeholder match/game (stub). Use real CSV with time,agent_id,x,y for full load.".to_string();
                // Reuse demo data for immediate viz feedback
                app.seed_spatial_demo();
                app.pending_spatial_import = false;
                app.spatial_view = crate::app::SpatialView::Visualize;
                return false;
            }
            KeyCode::Esc => {
                app.pending_spatial_import = false;
                app.status = "Spatial import/generate cancelled".to_string();
                return false;
            }
            _ => { return false; }
        }
    }

    // Quick toggle sub views (G/I/V)
    match code {
        KeyCode::Char('g') | KeyCode::Char('G') => {
            if app.spatial_view != crate::app::SpatialView::Generate {
                app.spatial_view = crate::app::SpatialView::Generate;
            } else {
                app.seed_spatial_demo();
                app.spatial_view = crate::app::SpatialView::Visualize;
                app.status = "Spatial: regenerated synthetic demo".to_string();
            }
            return false;
        }
        KeyCode::Char('i') | KeyCode::Char('I') => {
            app.spatial_view = crate::app::SpatialView::ImportData;
            app.pending_spatial_import = true;
            app.status = "Spatial import: 1=synth regen  2/ i =load placeholder match  Esc=cancel".to_string();
            return false;
        }
        KeyCode::Char('v') | KeyCode::Char('V') => {
            app.spatial_view = crate::app::SpatialView::Visualize;
            app.pending_spatial_import = false;
            app.status = "Spatial: visualize mode (arrows n/p < > 1-9)".to_string();
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
                if let Some((f, _)) = app.spatial_events.iter().rev().find(|(f, _)| *f < app.spatial_frame_idx) {
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
            KeyCode::Char('g') | KeyCode::Char('G') => {
                app.seed_spatial_demo();
                app.status = "Spatial: Regenerated synthetic demo".to_string();
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                if let (Some(batch), Some(focal_vec)) = (&app.spatial_batch, &app.spatial_focal) {
                    let maxf = batch.num_times().saturating_sub(1);
                    let idx = app.spatial_frame_idx.min(maxf);
                    let fpos = focal_vec.get(idx).copied();
                    if let Some(carrier) = batch.infer_ball_carrier_at(idx, fpos) {
                        let extra = app.spatial_decisions.as_ref()
                            .and_then(|decs| decs.get(carrier).and_then(|r| r.get(idx)))
                            .map(|d| {
                                format!(" spd={:.1} fwd={:+.2} conf={:.2}",
                                        d.features.speed, d.features.forward_component,
                                        d.confidence.unwrap_or(0.0))
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
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if let Some(digit) = c.to_digit(10) {
                    let ev_idx = digit as usize;
                    if ev_idx < app.spatial_events.len() {
                        let (frame, desc) = &app.spatial_events[ev_idx];
                        app.spatial_frame_idx = *frame;
                        app.status = format!("Spatial: jumped to event {} '{}' (frame {})", ev_idx, desc, frame);
                    } else if !app.spatial_events.is_empty() {
                        app.status = format!("Spatial: no event {} (have 0-{})", ev_idx, app.spatial_events.len()-1);
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
        let maxf = app.spatial_batch.as_ref().map(|b| b.num_times().saturating_sub(1)).unwrap_or(0);
        let _ev_hint = if !app.spatial_events.is_empty() { " | <>/1-9 events" } else { "" };
        app.status = format!("Spatial: frame {}/{}", app.spatial_frame_idx, maxf);
    }

    false
}

fn handle_import_keys(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
    if app.pending_generate {
        match code {
            KeyCode::Char('1') => {
                if let Err(e) = crate::processing::generate_demo_and_load(app, crate::generate::DemoPreset::RestingPPG) {
                    app.status = format!("Generation failed: {e}");
                }
                app.pending_generate = false;
                app.manual_path.clear();
                return false;
            }
            KeyCode::Char('2') => {
                if let Err(e) = crate::processing::generate_demo_and_load(app, crate::generate::DemoPreset::LightRespiration) {
                    app.status = format!("Generation failed: {e}");
                }
                app.pending_generate = false;
                app.manual_path.clear();
                return false;
            }
            KeyCode::Char('3') => {
                if let Err(e) = crate::processing::generate_demo_and_load(app, crate::generate::DemoPreset::SimpleStride) {
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
        match code {
            KeyCode::Char('1') => {
                app.process_selection = 0;
                app.status = "Process: Moving Average selected. ←/→ or -/+ to adjust window, Enter to apply.".to_string();
            }
            KeyCode::Char('2') => {
                app.process_selection = 1;
                app.status = "Process: Median Filter selected. ←/→ or -/+ to adjust window, Enter to apply.".to_string();
            }
            KeyCode::Char('3') => {
                app.process_selection = 2;
                app.status = "Process: Detrend selected. Enter to apply.".to_string();
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
        KeyCode::Char('p') | KeyCode::Char('P') => {
            app.pending_process = true;
            app.status = "Process: 1=MA 2=Median 3=Detrend   ←/→ adjust   Enter=Apply   Esc=Cancel".to_string();
            return false;
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if let Some(signal) = &mut app.loaded_signal {
                signal.reset();
                app.status = "Reset to original.".to_string();
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
                if app.rqa_params.radius > 0.05 { app.rqa_params.radius -= 0.05; }
                app.status = format!("RQA radius: {:.2}", app.rqa_params.radius);
            }
            KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('=') => {
                app.rqa_params.radius += 0.05;
                app.status = format!("RQA radius: {:.2}", app.rqa_params.radius);
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
                    let res = symworx_dynamics::rqa(&sig.current, app.rqa_params.m, app.rqa_params.tau, app.rqa_params.radius, app.rqa_params.theiler);
                    app.last_rqa = Some(res);
                    app.status = "RQA computed. See Dynamics tab for DET, RR, etc + plot.".to_string();
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
            app.status = format!("RQA params: m={} tau={} rad={:.2}  ←→ rad  m/t dim/delay  Enter=compute  Esc=cancel", app.rqa_params.m, app.rqa_params.tau, app.rqa_params.radius);
            return false;
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            app.rqa_params = crate::app::RqaParams::default();
            app.last_rqa = None;
            app.status = "RQA params reset to defaults".to_string();
            return false;
        }
        _ => {}
    }
    false
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
                app.status = "LoadSym: synthetic stub (placeholder).".to_string();
            }
            return false;
        }
        KeyCode::Char('i') | KeyCode::Char('I') => {
            if app.home_selection == 1 {
                app.status = "LoadSym: import stub (placeholder).".to_string();
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
