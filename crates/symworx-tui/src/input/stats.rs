use crossterm::event::{
    KeyCode,
    KeyModifiers,
};

use super::common::{
    arm_pending_delete,
    handle_pending_delete_keys,
};
use crate::app::App;

pub fn try_load_stats_table(app: &mut App, path: &str) {
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

pub fn handle_stats_keys(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
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

pub fn run_stats_generate(app: &mut App) {
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
