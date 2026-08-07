// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! LoadSym catalog, calendar, workout open, and plan helpers.

// ---------------------------------------------------------------------------
// LoadSym helpers (activity discovery + load derivation for calendar)
// ---------------------------------------------------------------------------

use symworx_loadsym::load::compute_ride_metrics_from_activity;

use crate::app::{
    ActivityMetricsUiRow,
    App,
    CatalogRideRow,
    WeeklyLoadRow,
};

/// Count discoverable activity files under the app's archive dirs (paths only; no FIT parse).
pub fn count_loadsym_activity_files(app: &App) -> usize {
    symworx_io::discover_activity_files(&app.loadsym_archive_dirs, false).len()
}

/// Scan archive dirs for the newest usable activity (by mtime).
/// Prefers `~/velofit/inbox` + `~/velofit/raw` when present (see `loadsym_archive_dirs`).
pub fn find_newest_loadsym_activity(app: &App) -> Option<symworx_io::ActivityData> {
    let entries = symworx_io::discover_activity_files(&app.loadsym_archive_dirs, false);
    for e in entries {
        if let Ok(act) = symworx_io::load_activity(&e.path.to_string_lossy()) {
            if !act.times_s.is_empty() {
                return Some(act);
            }
        }
    }
    None
}

/// Backward-compatible alias used by older call sites.
pub fn find_first_loadsym_activity(app: &App) -> Option<symworx_io::ActivityData> {
    find_newest_loadsym_activity(app)
}

/// Derive a daily load value (TSS preferred) for a loaded activity using current FTP.
pub fn derive_load_from_current_activity(app: &App) -> Option<f64> {
    app.loaded_activity.as_ref().map(|act| {
        let p = act.power_w.clone();
        let m = compute_ride_metrics_from_activity(&act.times_s, &p, app.ftp);
        m.tss.max(1.0) // at least a token load
    })
}

/// Load daily TSS / ACWR / rides from the personal SQLite catalog.
pub fn try_load_loadsym_catalog(app: &mut App) -> Result<bool, String> {
    match symworx_loadsym::catalog::try_load_default_calendar()? {
        None => Ok(false),
        Some((path, rows, rides)) => {
            if rows.is_empty() {
                app.loadsym_catalog_path = Some(path);
                app.loadsym_from_catalog = false;
                app.catalog_rides.clear();
                app.catalog_activity_metrics.clear();
                app.metrics_scroll = 0;
                app.weekly_loads.clear();
                return Ok(false);
            }
            app.daily_loads = rows.iter().map(|r| r.total_tss).collect();
            app.daily_load_dates = rows.iter().map(|r| r.ride_date.clone()).collect();
            app.daily_acwr = rows.iter().map(|r| r.acwr).collect();
            app.daily_risk = rows.iter().map(|r| r.risk_level.clone()).collect();
            app.daily_ride_counts = rows.iter().map(|r| r.ride_count).collect();
            app.catalog_rides = rides
                .into_iter()
                .map(|r| CatalogRideRow {
                    ride_date: r.ride_date,
                    source_file: r.source_file,
                    tss: r.tss,
                    duration_s: r.duration_s,
                    np_w: r.np_w,
                    ingest_pipeline: r.ingest_pipeline,
                    source_platform: r.source_platform,
                    counts_for_load: r.counts_for_load,
                    is_primary: r.is_primary,
                })
                .collect();
            app.weekly_loads = build_weekly_loads(&app.daily_load_dates, &app.daily_loads, &app.daily_ride_counts);
            app.loadsym_catalog_path = Some(path);
            app.loadsym_from_catalog = true;
            // Metrics table (best-effort; empty if query fails)
            if let Ok(Some((_, metrics))) = symworx_loadsym::catalog::try_load_default_activity_metrics() {
                app.catalog_activity_metrics = metrics
                    .into_iter()
                    .map(|r| ActivityMetricsUiRow {
                        id: r.id,
                        ride_date: r.ride_date,
                        source_file: r.source_file,
                        duration_s: r.duration_s,
                        sport: r.sport,
                        avg_power_w: r.avg_power_w,
                        max_power_w: r.max_power_w,
                        np_w: r.np_w,
                        intensity_factor: r.intensity_factor,
                        tss: r.tss,
                        total_work_kj: r.total_work_kj,
                        avg_hr_bpm: r.avg_hr_bpm,
                        max_hr_bpm: r.max_hr_bpm,
                        ftp_used_w: r.ftp_used_w,
                    })
                    .collect();
                app.metrics_scroll = app.catalog_activity_metrics.len().saturating_sub(1);
            } else {
                app.catalog_activity_metrics.clear();
                app.metrics_scroll = 0;
            }
            invalidate_loadsym_plan_cache(app);
            focus_calendar_most_recent(app);
            Ok(true)
        }
    }
}

/// Open the focused Metrics-table ride in Workout Analysis.
pub fn open_metrics_row_into_workout(app: &mut App) -> bool {
    if app.catalog_activity_metrics.is_empty() {
        app.status = "No activity metrics — r to reload catalog".to_string();
        return false;
    }
    let idx = app
        .metrics_scroll
        .min(app.catalog_activity_metrics.len().saturating_sub(1));
    let row = app.catalog_activity_metrics[idx].clone();
    let Some(path) = resolve_activity_path(&row.source_file, &app.loadsym_archive_dirs) else {
        app.status = format!("Cannot resolve {} ({})", row.ride_date, row.source_file);
        return false;
    };
    match load_activity_into_app(app, &path) {
        Ok(msg) => {
            app.loadsym_view = crate::app::LoadSymView::Workout;
            app.status = format!(
                "{} · from metrics {} TSLi={}",
                msg,
                row.ride_date,
                row.tss.map(|t| format!("{:.0}", t)).unwrap_or_else(|| "-".into())
            );
            true
        }
        Err(e) => {
            app.status = e;
            false
        }
    }
}

/// Apply synthetic demo loads (clears catalog-backed date metadata).
pub fn apply_demo_daily_loads(app: &mut App, days: usize) {
    app.daily_loads = symworx_loadsym::load::generate_demo_daily_loads(days, 400.0, 100.0);
    app.daily_load_dates = (0..app.daily_loads.len()).map(|i| format!("d{:03}", i)).collect();
    app.daily_acwr.clear();
    app.daily_risk.clear();
    app.daily_ride_counts = vec![1; app.daily_loads.len()];
    app.catalog_rides.clear();
    app.catalog_activity_metrics.clear();
    app.metrics_scroll = 0;
    app.weekly_loads = build_weekly_loads(&app.daily_load_dates, &app.daily_loads, &app.daily_ride_counts);
    app.loadsym_from_catalog = false;
    app.loadsym_catalog_path = None;
    invalidate_loadsym_plan_cache(app);
    focus_calendar_most_recent(app);
}

/// Fingerprint of plan inputs (goal, horizon, daily TSS series).
pub fn loadsym_plan_input_key(app: &App) -> u64 {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{
            Hash,
            Hasher,
        },
    };
    let mut h = DefaultHasher::new();
    app.loadsym_plan_goal.as_str().hash(&mut h);
    app.loadsym_plan_horizon.hash(&mut h);
    app.daily_loads.len().hash(&mut h);
    for &v in &app.daily_loads {
        v.to_bits().hash(&mut h);
    }
    h.finish()
}

pub fn invalidate_loadsym_plan_cache(app: &mut App) {
    app.loadsym_cached_plan = None;
    app.loadsym_cached_plan_err = None;
    app.loadsym_plan_cache_key = 0;
}

/// Recompute plan only when goal / horizon / loads changed (not every TUI frame).
pub fn ensure_loadsym_plan(app: &mut App) {
    use symworx_loadsym::load::{
        MAX_HORIZON_DAYS,
        OptimizationThresholds,
        PulseResponseParams,
        optimize_load_plan,
    };
    if app.daily_loads.is_empty() {
        invalidate_loadsym_plan_cache(app);
        return;
    }
    let key = loadsym_plan_input_key(app);
    if key == app.loadsym_plan_cache_key && (app.loadsym_cached_plan.is_some() || app.loadsym_cached_plan_err.is_some())
    {
        return;
    }
    let horizon = app.loadsym_plan_horizon.clamp(2, MAX_HORIZON_DAYS);
    app.loadsym_plan_horizon = horizon;
    let thr = OptimizationThresholds {
        horizon_days: horizon,
        ..Default::default()
    };
    let params = PulseResponseParams::pmc_defaults();
    match optimize_load_plan(&app.daily_loads, &params, app.loadsym_plan_goal, &thr) {
        Ok(plan) => {
            app.loadsym_cached_plan = Some(plan);
            app.loadsym_cached_plan_err = None;
        }
        Err(e) => {
            app.loadsym_cached_plan = None;
            app.loadsym_cached_plan_err = Some(e.to_string());
        }
    }
    app.loadsym_plan_cache_key = key;
}

/// Jump calendar focus to the most recent day (and its week).
pub fn focus_calendar_most_recent(app: &mut App) {
    if !app.daily_loads.is_empty() {
        app.loadsym_scroll = app.daily_loads.len() - 1;
    } else {
        app.loadsym_scroll = 0;
    }
    sync_week_scroll_from_daily(app);
    // Prefer newest week when weekly series exists
    if !app.weekly_loads.is_empty() {
        app.loadsym_week_scroll = app.weekly_loads.len() - 1;
        // Keep daily on last day of that week (should match most recent day)
        app.loadsym_scroll = app.weekly_loads[app.loadsym_week_scroll].day_index_hi;
    }
    app.loadsym_scroll_from_week = false;
    app.calendar_ride_sel = 0;
}

/// Build Mon–Sun weeks from daily series.
pub fn build_weekly_loads(dates: &[String], loads: &[f64], ride_counts: &[i64]) -> Vec<WeeklyLoadRow> {
    if dates.is_empty() || loads.is_empty() {
        return vec![];
    }
    let mut weeks: Vec<WeeklyLoadRow> = Vec::new();
    let mut i = 0usize;
    while i < loads.len() {
        let (week_start, group_end) = if let Some(ws) = week_start_ymd(&dates[i]) {
            let mut j = i + 1;
            while j < dates.len() {
                match week_start_ymd(&dates[j]) {
                    Some(w) if w == ws => j += 1,
                    _ => break,
                }
            }
            (ws, j)
        } else {
            let j = (i + 7).min(loads.len());
            (format!("W{}", i / 7), j)
        };
        let slice = i..group_end;
        let total_tss: f64 = loads[slice.clone()].iter().sum();
        let ride_count: i64 = ride_counts.get(slice.clone()).map(|s| s.iter().sum()).unwrap_or(0);
        weeks.push(WeeklyLoadRow {
            week_start,
            total_tss,
            ride_count,
            day_count: group_end - i,
            day_index_lo: i,
            day_index_hi: group_end.saturating_sub(1),
        });
        i = group_end;
    }
    weeks
}

fn week_start_ymd(ymd: &str) -> Option<String> {
    let (y, m, d) = parse_ymd(ymd)?;
    let wd = weekday_mon0(y, m, d)?;
    let mut yy = y;
    let mut mm = m;
    let mut dd = d as i32 - wd as i32;
    while dd < 1 {
        mm -= 1;
        if mm < 1 {
            mm = 12;
            yy -= 1;
        }
        dd += days_in_month(yy, mm) as i32;
    }
    Some(format!("{:04}-{:02}-{:02}", yy, mm, dd))
}

fn parse_ymd(s: &str) -> Option<(i32, u32, u32)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    Some((parts[0].parse().ok()?, parts[1].parse().ok()?, parts[2].parse().ok()?))
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn weekday_mon0(y: i32, m: u32, d: u32) -> Option<u32> {
    let t = [0i32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let mut y = y;
    if m < 3 {
        y -= 1;
    }
    let w = (y + y / 4 - y / 100 + y / 400 + t[(m - 1) as usize] + d as i32) % 7;
    Some(((w + 6) % 7) as u32)
}

pub fn sync_week_scroll_from_daily(app: &mut App) {
    app.loadsym_scroll_from_week = false;
    if app.weekly_loads.is_empty() {
        app.loadsym_week_scroll = 0;
        return;
    }
    let day = app.loadsym_scroll.min(app.daily_loads.len().saturating_sub(1));
    if let Some((wi, _)) = app
        .weekly_loads
        .iter()
        .enumerate()
        .find(|(_, w)| day >= w.day_index_lo && day <= w.day_index_hi)
    {
        app.loadsym_week_scroll = wi;
    } else {
        app.loadsym_week_scroll = app.weekly_loads.len().saturating_sub(1);
    }
}

pub fn sync_daily_scroll_from_week(app: &mut App) {
    app.loadsym_scroll_from_week = true;
    if app.weekly_loads.is_empty() {
        return;
    }
    let wi = app.loadsym_week_scroll.min(app.weekly_loads.len().saturating_sub(1));
    app.loadsym_week_scroll = wi;
    app.loadsym_scroll = app.weekly_loads[wi].day_index_lo;
}

pub fn rides_for_focus_day(app: &App) -> Vec<&CatalogRideRow> {
    let date = match app.daily_load_dates.get(app.loadsym_scroll) {
        Some(d) => d.as_str(),
        None => return vec![],
    };
    app.catalog_rides.iter().filter(|r| r.ride_date == date).collect()
}

/// Clamp `calendar_ride_sel` to the rides available on the focused day.
pub fn clamp_calendar_ride_sel(app: &mut App) {
    let n = rides_for_focus_day(app).len();
    if n == 0 {
        app.calendar_ride_sel = 0;
    } else {
        app.calendar_ride_sel = app.calendar_ride_sel.min(n - 1);
    }
}

/// Resolve a catalog `source_file` key (absolute, relative to VELOFIT, or basename search).
pub fn resolve_activity_path(source_key: &str, search_dirs: &[std::path::PathBuf]) -> Option<std::path::PathBuf> {
    use std::path::{
        Path,
        PathBuf,
    };
    let key = source_key.trim();
    if key.is_empty() {
        return None;
    }
    let as_path = PathBuf::from(key);
    if as_path.is_absolute() && as_path.is_file() {
        return Some(as_path);
    }
    if as_path.is_file() {
        return Some(as_path);
    }

    let root = symworx_io::default_velofit_root();
    let candidates = [
        root.join(key),
        symworx_io::default_velofit_raw().join(key),
        symworx_io::default_velofit_inbox().join(key),
        root.join("raw").join(key),
        root.join("inbox").join(key),
    ];
    for c in candidates {
        if c.is_file() {
            return Some(c);
        }
    }

    // Basename-only match under search dirs (non-recursive + one recursive pass on velofit)
    let base = Path::new(key).file_name().and_then(|s| s.to_str()).unwrap_or(key);
    let entries = symworx_io::discover_activity_files(search_dirs, false);
    for e in &entries {
        if e.path.file_name().and_then(|n| n.to_str()) == Some(base) {
            return Some(e.path.clone());
        }
    }
    // Recursive under velofit root if present (S3 layouts can nest)
    if root.is_dir() {
        let nested = symworx_io::discover_activity_files(&[root], true);
        for e in nested {
            if e.path.file_name().and_then(|n| n.to_str()) == Some(base) {
                return Some(e.path);
            }
        }
    }
    None
}

/// Load an activity file into the Workout analyzer state.
///
/// **Panel visibility:** if an activity is already loaded (reload via `i` / `o` /
/// calendar), keep the user’s current `workout_stream_on` selection. Only the
/// first open of a session (no prior activity) defaults to “all present streams”.
pub fn load_activity_into_app(app: &mut App, path: &std::path::Path) -> Result<String, String> {
    let path_str = path.to_string_lossy();
    let act = symworx_io::load_activity(&path_str).map_err(|e| format!("load failed: {e}"))?;
    if act.is_empty() {
        return Err(format!("no samples in {}", path.display()));
    }
    let n = act.len();
    let src = act.source.clone();
    use crate::app::WorkoutStream;

    // Preserve user panel layout when reloading (e.g. `i` after closing elevation).
    let preserve_panels = app.loaded_activity.is_some();
    let mut on = if preserve_panels {
        app.workout_stream_on
    } else {
        // First load: open every channel that has data.
        let mut on = [false; WorkoutStream::COUNT];
        for s in WorkoutStream::ALL {
            on[s.index()] = s.present_on(&act);
        }
        on
    };
    // Always keep at least one panel open.
    if !on.iter().any(|&v| v) {
        // Prefer a present stream, else power.
        let fallback = WorkoutStream::ALL
            .iter()
            .find(|s| s.present_on(&act))
            .copied()
            .unwrap_or(WorkoutStream::Power);
        on[fallback.index()] = true;
    }

    // Focus stats: keep previous focus on reload if still open; else first present stream.
    let series = if preserve_panels {
        let prev = WorkoutStream::from_index(app.activity_series).unwrap_or(WorkoutStream::Power);
        if on[prev.index()] {
            prev
        } else {
            WorkoutStream::ALL
                .iter()
                .copied()
                .find(|s| on[s.index()])
                .unwrap_or(WorkoutStream::Power)
        }
    } else if act.has_power() {
        WorkoutStream::Power
    } else if act.has_hr() {
        WorkoutStream::HeartRate
    } else if act.has_speed() {
        WorkoutStream::Speed
    } else if act.has_cadence() {
        WorkoutStream::Cadence
    } else {
        WorkoutStream::Elevation
    };

    app.loaded_activity = Some(act);
    app.activity_scroll = 0;
    app.activity_series = series.index();
    app.workout_stream_on = on;
    // Keep thresh/dur on reload so exploration state survives `i`.
    if !preserve_panels {
        app.workout_user_thresh = 0.0;
        app.workout_user_min_dur = 3;
    }
    app.pending_workout_open = false;
    let open_n = on.iter().filter(|&&v| v).count();
    Ok(format!(
        "Loaded {} ({} samples) · {} panel(s) kept  1–5 toggle",
        src, n, open_n
    ))
}

/// Toggle a workout chart panel; refuses to close the last open panel.
/// Returns a status string.
pub fn toggle_workout_panel(app: &mut App, which: u8) -> String {
    use crate::app::WorkoutStream;
    let Some(stream) = WorkoutStream::from_index(which as usize) else {
        return "Unknown stream (use 1–5)".into();
    };
    let idx = stream.index();
    let open_count = app.workout_stream_on.iter().filter(|&&v| v).count();
    let currently = app.workout_stream_on[idx];
    let name = stream.short_label();

    if currently && open_count <= 1 {
        return format!("Keep at least one panel open ({name})");
    }

    // Optional: warn if enabling a channel with no data (still allowed so user can see empty).
    let has_data = app
        .loaded_activity
        .as_ref()
        .map(|a| stream.present_on(a))
        .unwrap_or(false);

    app.workout_stream_on[idx] = !currently;
    app.activity_series = idx;
    let now = app.workout_stream_on[idx];
    if now && !has_data {
        format!("Panel {name}: shown (no data in file)  ·  1–5 toggle · height redistributes")
    } else {
        format!(
            "Panel {name}: {}  ·  1–5 toggle · remaining fill height",
            if now { "shown" } else { "hidden" }
        )
    }
}

/// Open the currently selected calendar ride into Workout Analysis.
/// Returns true if navigation to Workout succeeded.
pub fn open_calendar_ride_into_workout(app: &mut App) -> bool {
    clamp_calendar_ride_sel(app);
    let rides: Vec<CatalogRideRow> = rides_for_focus_day(app).into_iter().cloned().collect();
    if rides.is_empty() {
        app.status = "No ride files on this day (demo days have none — use catalog + Enter/o)".to_string();
        return false;
    }
    let ride = &rides[app.calendar_ride_sel];
    let Some(path) = resolve_activity_path(&ride.source_file, &app.loadsym_archive_dirs) else {
        app.status = format!("Cannot resolve file for {} ({})", ride.ride_date, ride.source_file);
        return false;
    };
    match load_activity_into_app(app, &path) {
        Ok(msg) => {
            app.loadsym_view = crate::app::LoadSymView::Workout;
            app.status = format!("{} · from calendar {} TSLi={:.1}", msg, ride.ride_date, ride.tss);
            true
        }
        Err(e) => {
            app.status = e;
            false
        }
    }
}

/// Populate the workout open-file modal list (newest first).
pub fn refresh_workout_file_list(app: &mut App) {
    let entries = symworx_io::discover_activity_files(&app.loadsym_archive_dirs, false);
    app.workout_file_list = entries.into_iter().map(|e| e.path).collect();
    app.workout_file_sel = 0;
}

/// Open selected path from the workout file browser.
pub fn open_selected_workout_file(app: &mut App) -> bool {
    if app.workout_file_list.is_empty() {
        app.status = "No activity files in $VELOFIT_HOME/raw|inbox or ./data".to_string();
        app.pending_workout_open = false;
        return false;
    }
    let idx = app.workout_file_sel.min(app.workout_file_list.len().saturating_sub(1));
    let path = app.workout_file_list[idx].clone();
    match load_activity_into_app(app, &path) {
        Ok(msg) => {
            app.status = msg;
            true
        }
        Err(e) => {
            app.status = e;
            false
        }
    }
}

/// Suggest planning goal from form/fatigue/ACLi when the user has not overridden.
///
/// `force` ignores `loadsym_goal_user_override` (used on first enter).
pub fn apply_suggested_load_goal(app: &mut App, force: bool) {
    use symworx_loadsym::load::{
        GoalSuggestParams,
        PulseResponseParams,
        compute_acute_chronic,
        simulate_pulse_response,
        suggest_load_goal,
    };
    if app.daily_loads.is_empty() {
        app.loadsym_goal_suggest_note.clear();
        return;
    }
    if !force && app.loadsym_goal_user_override {
        return;
    }
    let params = PulseResponseParams::pmc_defaults();
    let Ok(series) = simulate_pulse_response(&app.daily_loads, &params, None) else {
        app.loadsym_goal_suggest_note = "suggest: pulse-response failed".into();
        return;
    };
    let Some(state) = series.last_state() else {
        return;
    };
    let acwr = compute_acute_chronic(&app.daily_loads, 7, 28).ok().map(|s| s.acwr);
    let suggestion = suggest_load_goal(&state, acwr, &GoalSuggestParams::default());
    app.loadsym_plan_goal = suggestion.goal;
    app.loadsym_goal_suggest_note = format!(
        "suggested {} ({:.0}% conf) · form {:+.0} · 1/2/3 override",
        suggestion.goal.as_str(),
        suggestion.confidence * 100.0,
        state.form
    );
    if force {
        app.loadsym_goal_user_override = false;
    }
    invalidate_loadsym_plan_cache(app);
}
