use crate::{
    app::{App, Tab},
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
        io::{BufRead, BufReader},
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

/// Scan candidate dirs (data + app archive dirs) for first usable activity.
/// (Extended version of the one in input for reuse.)
pub fn find_first_loadsym_activity(app: &App) -> Option<symworx_io::ActivityData> {
    let mut dirs: Vec<std::path::PathBuf> = vec!["data".into(), "rides".into(), "training".into()];
    dirs.extend(app.loadsym_archive_dirs.iter().cloned());
    for d in dirs {
        if !d.exists() {
            continue;
        }
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_file() {
                    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                        let el = ext.to_lowercase();
                        if matches!(el.as_str(), "fit" | "csv" | "txt") {
                            if let Ok(act) = symworx_io::load_activity(&p.to_string_lossy()) {
                                if !act.times_s.is_empty() {
                                    return Some(act);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Derive a daily load value (TSS preferred) for a loaded activity using current FTP.
pub fn derive_load_from_current_activity(app: &App) -> Option<f64> {
    app.loaded_activity.as_ref().map(|act| {
        let p = act.power_w.clone();
        let m = compute_ride_metrics_from_activity(&act.times_s, &p, app.ftp);
        m.tss.max(1.0) // at least a token load
    })
}
