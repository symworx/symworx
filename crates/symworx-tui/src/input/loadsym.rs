use crossterm::event::{
    KeyCode,
    KeyModifiers,
};

use crate::app::App;

pub fn calendar_status(app: &App) -> String {
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

pub fn handle_loadsym_keys(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
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
pub fn handle_workout_open_modal(app: &mut App, code: KeyCode) -> bool {
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

pub fn enter_loadsym_optimization(app: &mut App) {
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

pub fn enter_loadsym_metrics(app: &mut App) {
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

/// Switch to a LoadSym footer-strip view (Ctrl+←→). Runs enter-side effects for catalog views.
pub fn apply_loadsym_strip_view(app: &mut App, view: crate::app::LoadSymView) {
    use crate::app::LoadSymView;
    // List is hub-only; strip cycle never targets it.
    let view = if view == LoadSymView::List {
        LoadSymView::Workout
    } else {
        view
    };
    match view {
        LoadSymView::Workout => {
            app.loadsym_view = LoadSymView::Workout;
            app.status =
                "Workout: o open file  i newest  1–5 streams  ←→ pan  Esc list  ·  Ctrl+←→ views"
                    .to_string();
        }
        LoadSymView::Metrics => enter_loadsym_metrics(app),
        LoadSymView::Calendar => {
            app.loadsym_view = LoadSymView::Calendar;
            let _ = crate::processing::try_load_loadsym_catalog(app);
            crate::processing::focus_calendar_most_recent(app);
            crate::processing::clamp_calendar_ride_sel(app);
            app.status = calendar_status(app);
        }
        LoadSymView::Optimization => enter_loadsym_optimization(app),
        LoadSymView::List => unreachable!("mapped to Workout above"),
    }
}

pub fn metrics_status(app: &App) -> String {
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

pub fn opt_status(app: &App) -> String {
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
