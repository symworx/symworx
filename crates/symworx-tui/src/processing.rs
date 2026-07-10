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

/// Load daily TSS / ACWR from the personal SQLite catalog (`$VELOFIT_HOME/db/…`).
///
/// Returns `Ok(true)` if rows were loaded, `Ok(false)` if no DB file, `Err` on I/O/SQL errors.
pub fn try_load_loadsym_catalog(app: &mut App) -> Result<bool, String> {
    match symworx_loadsym::catalog::try_load_default_calendar()? {
        None => {
            // Leave existing series alone if catalog missing
            Ok(false)
        }
        Some((path, rows)) => {
            if rows.is_empty() {
                app.loadsym_catalog_path = Some(path);
                app.loadsym_from_catalog = false;
                return Ok(false);
            }
            app.daily_loads = rows.iter().map(|r| r.total_tss).collect();
            app.daily_load_dates = rows.iter().map(|r| r.ride_date.clone()).collect();
            app.daily_acwr = rows.iter().map(|r| r.acwr).collect();
            app.daily_risk = rows.iter().map(|r| r.risk_level.clone()).collect();
            app.daily_ride_counts = rows.iter().map(|r| r.ride_count).collect();
            app.loadsym_catalog_path = Some(path);
            app.loadsym_from_catalog = true;
            // Focus on most recent day
            app.loadsym_scroll = app.daily_loads.len().saturating_sub(1);
            Ok(true)
        }
    }
}

/// Apply synthetic demo loads (clears catalog-backed date metadata).
pub fn apply_demo_daily_loads(app: &mut App, days: usize) {
    app.daily_loads = symworx_loadsym::load::generate_demo_daily_loads(days, 400.0, 100.0);
    app.daily_load_dates.clear();
    app.daily_acwr.clear();
    app.daily_risk.clear();
    app.daily_ride_counts.clear();
    app.loadsym_from_catalog = false;
    app.loadsym_catalog_path = None;
    app.loadsym_scroll = app.daily_loads.len().saturating_sub(1);
}
