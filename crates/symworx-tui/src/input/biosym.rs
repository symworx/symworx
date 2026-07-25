use crossterm::event::{
    KeyCode,
    KeyModifiers,
};

use super::common::{
    arm_pending_delete,
    handle_pending_delete_keys,
};
use crate::app::{
    App,
    Tab,
};

pub fn run_bio_generate(app: &mut App, preset: crate::generate::DemoPreset) {
    app.pending_generate = false;
    app.manual_path.clear();
    if let Err(e) = crate::processing::generate_demo_and_load(app, preset) {
        app.status = format!("Generation failed: {e}");
    }
}

pub fn handle_bio_generate_keys(app: &mut App, code: KeyCode) -> bool {
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

pub fn handle_import_keys(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
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

pub fn handle_explore_keys(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
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

pub fn handle_dynamics_keys(app: &mut App, code: KeyCode) -> bool {
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

pub fn export_rqa_csv(
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
