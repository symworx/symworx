use crate::app::{App, Tab};
use crossterm::event::{KeyCode, KeyModifiers};
use anyhow::Result;

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
        }
        return false;
    }

    match code {
        KeyCode::Char('1') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_tab = Tab::Import;
            app.ensure_status_for_current_tab();
            return false;
        }
        KeyCode::Char('2') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_tab = Tab::Explore;
            app.ensure_status_for_current_tab();
            return false;
        }
        KeyCode::Char('3') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_tab = Tab::Dynamics;
            app.ensure_status_for_current_tab();
            return false;
        }
        KeyCode::Char('4') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_tab = Tab::Spatial;
            app.ensure_status_for_current_tab();
            return false;
        }
        KeyCode::Left if modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_tab = match app.current_tab {
                Tab::Import => Tab::Import,
                Tab::Explore => Tab::Import,
                Tab::Dynamics => Tab::Explore,
                Tab::Spatial => Tab::Dynamics,
            };
            app.ensure_status_for_current_tab();
            return false;
        }
        KeyCode::Right if modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_tab = match app.current_tab {
                Tab::Import => Tab::Explore,
                Tab::Explore => Tab::Dynamics,
                Tab::Dynamics => Tab::Spatial,
                Tab::Spatial => Tab::Spatial,
            };
            app.ensure_status_for_current_tab();
            return false;
        }
        KeyCode::Char('g') | KeyCode::Char('G') if modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_tab != Tab::Import {
                app.current_tab = Tab::Import;
            }
            app.pending_generate = true;
            app.manual_path.clear();
            app.file_filter.clear();
            app.status = "Generate demo data: 1 = Resting PPG   2 = Respiration   3 = Stride intervals   Esc = cancel".to_string();
            app.ensure_status_for_current_tab();
            return false;
        }
        _ => {}
    }

    match app.current_tab {
        Tab::Import => handle_import_keys(app, code, modifiers),
        Tab::Explore => handle_explore_keys(app, code, modifiers),
        Tab::Dynamics => handle_dynamics_keys(app, code),
        Tab::Spatial => handle_spatial_keys(app, code, modifiers),
    }
}

fn handle_spatial_keys(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
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
        let ev_hint = if !app.spatial_events.is_empty() { " | <>/1-9 events" } else { "" };
        app.status = format!("Spatial: frame {}/{}", app.spatial_frame_idx, maxf);
    }

    false
}

fn handle_import_keys(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
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
                app.status = "Generate cancelled".to_string();
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

fn handle_explore_keys(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
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

fn handle_dynamics_keys(_app: &mut App, _code: KeyCode) -> bool {
    false
}
