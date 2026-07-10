use crate::{
    app::{
        App,
        Tab,
    },
    generate,
};

pub fn moving_average(data: &[f64], window: usize) -> Vec<f64> {
    if window == 0 {
        return data.to_vec();
    }
    let mut out = Vec::with_capacity(data.len());
    for i in 0..data.len() {
        let start = i.saturating_sub(window / 2);
        let end = (i + window / 2 + 1).min(data.len());
        let sum: f64 = data[start..end].iter().sum();
        out.push(sum / (end - start) as f64);
    }
    out
}

pub fn median_filter(data: &[f64], window: usize) -> Vec<f64> {
    if window == 0 {
        return data.to_vec();
    }
    let mut out = Vec::with_capacity(data.len());
    for i in 0..data.len() {
        let start = i.saturating_sub(window / 2);
        let end = (i + window / 2 + 1).min(data.len());
        let mut w = data[start..end].to_vec();
        w.sort_by(|a, b| a.partial_cmp(b).unwrap());
        out.push(w[w.len() / 2]);
    }
    out
}

pub fn detrend_mean(data: &[f64]) -> Vec<f64> {
    if data.is_empty() {
        return vec![];
    }
    let mean = data.iter().sum::<f64>() / data.len() as f64;
    data.iter().map(|v| v - mean).collect()
}

pub fn generate_demo_and_load(app: &mut App, preset: generate::DemoPreset) -> anyhow::Result<()> {
    let data_dir = std::path::Path::new("data");
    let path = generate::generate_and_save(preset, data_dir)?;

    // Properly load generated BioSym files: skip header, take the signal column (last col, usually index 1 not time).
    // Generated files have headers and two columns: time,<signal>
    use std::{
        fs::File,
        io::{
            BufRead,
            BufReader,
        },
    };
    let file = File::open(&path)?;
    let reader = BufReader::new(file);
    let mut series = Vec::new();
    let mut has_header = false;
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if !has_header {
            // Header line typically contains non-numeric (or comma)
            if trimmed.contains(',') || trimmed.parse::<f64>().is_err() {
                has_header = true;
                continue;
            }
        }

        // Split on comma or whitespace; take the last token as the signal value (skip time col 0)
        let parts: Vec<&str> = trimmed
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .collect();
        if let Some(last) = parts.last() {
            if let Ok(v) = last.parse::<f64>() {
                series.push(v);
            }
        }
    }

    if series.is_empty() {
        anyhow::bail!("no numeric data in generated file");
    }

    app.loaded_signal = Some(crate::app::LoadedSignal::new(
        series,
        path.display().to_string(),
    ));
    app.explore_scroll = 0;
    app.current_tab = Tab::Explore;
    app.current_workflow = crate::app::Workflow::BioSym;
    app.status = format!(
        "Generated {} → loaded {} samples (BioSym signal col). Switched to Explore. (Ctrl+1=Import)",
        path.display(),
        app.loaded_signal.as_ref().map(|s| s.n_samples).unwrap_or(0)
    );
    app.ensure_status_for_current_tab();
    app.refresh_file_list();
    Ok(())
}

// ---------------------------------------------------------------------------
// LoadSym helpers (activity discovery + load derivation for calendar)
// ---------------------------------------------------------------------------

use symworx_loadsym::load::compute_ride_metrics_from_activity;

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

use crate::app::{CatalogRideRow, WeeklyLoadRow};

/// Load daily TSS / ACWR / rides from the personal SQLite catalog.
pub fn try_load_loadsym_catalog(app: &mut App) -> Result<bool, String> {
    match symworx_loadsym::catalog::try_load_default_calendar()? {
        None => Ok(false),
        Some((path, rows, rides)) => {
            if rows.is_empty() {
                app.loadsym_catalog_path = Some(path);
                app.loadsym_from_catalog = false;
                app.catalog_rides.clear();
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
                })
                .collect();
            app.weekly_loads =
                build_weekly_loads(&app.daily_load_dates, &app.daily_loads, &app.daily_ride_counts);
            app.loadsym_catalog_path = Some(path);
            app.loadsym_from_catalog = true;
            focus_calendar_most_recent(app);
            Ok(true)
        }
    }
}

/// Apply synthetic demo loads (clears catalog-backed date metadata).
pub fn apply_demo_daily_loads(app: &mut App, days: usize) {
    app.daily_loads = symworx_loadsym::load::generate_demo_daily_loads(days, 400.0, 100.0);
    app.daily_load_dates = (0..app.daily_loads.len())
        .map(|i| format!("d{:03}", i))
        .collect();
    app.daily_acwr.clear();
    app.daily_risk.clear();
    app.daily_ride_counts = vec![1; app.daily_loads.len()];
    app.catalog_rides.clear();
    app.weekly_loads =
        build_weekly_loads(&app.daily_load_dates, &app.daily_loads, &app.daily_ride_counts);
    app.loadsym_from_catalog = false;
    app.loadsym_catalog_path = None;
    focus_calendar_most_recent(app);
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
}

/// Build Mon–Sun weeks from daily series.
pub fn build_weekly_loads(
    dates: &[String],
    loads: &[f64],
    ride_counts: &[i64],
) -> Vec<WeeklyLoadRow> {
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
        let ride_count: i64 = ride_counts
            .get(slice.clone())
            .map(|s| s.iter().sum())
            .unwrap_or(0);
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
    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ))
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
    let wi = app
        .loadsym_week_scroll
        .min(app.weekly_loads.len().saturating_sub(1));
    app.loadsym_week_scroll = wi;
    app.loadsym_scroll = app.weekly_loads[wi].day_index_lo;
}

pub fn rides_for_focus_day(app: &App) -> Vec<&CatalogRideRow> {
    let date = match app.daily_load_dates.get(app.loadsym_scroll) {
        Some(d) => d.as_str(),
        None => return vec![],
    };
    app.catalog_rides
        .iter()
        .filter(|r| r.ride_date == date)
        .collect()
}
