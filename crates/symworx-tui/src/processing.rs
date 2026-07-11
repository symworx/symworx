use symworx_core::{
    PeakFinderBuilder,
    successive_differences,
};

use crate::{
    app::{
        App,
        LoadedSignal,
        PeakDetectParams,
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

/// First-order successive difference, padded with a leading 0 so `len == data.len()`.
///
/// Uses [`successive_differences`] from `symworx-math` (canonical series primitive).
/// Common first step before PPG peak detection on d(PPG)/dt.
pub fn first_derivative(data: &[f64]) -> Vec<f64> {
    if data.is_empty() {
        return vec![];
    }
    let diffs = successive_differences(data);
    let mut out = Vec::with_capacity(data.len());
    out.push(0.0);
    out.extend(diffs);
    out
}

/// Second-order difference (derivative of first derivative), same length as input.
///
/// Often used on PPG (d²x/dt²) for fiducial points / peak refinement.
pub fn second_derivative(data: &[f64]) -> Vec<f64> {
    first_derivative(&first_derivative(data))
}

/// Effective absolute thresholds for the current series + peak params.
///
/// Returns `(height, prominence, distance_samples, range, min, max)`.
pub fn peak_thresholds(
    data: &[f64],
    fs: Option<f64>,
    params: &PeakDetectParams,
) -> Option<(f64, f64, usize, f64, f64, f64)> {
    if data.len() < 3 {
        return None;
    }
    let min = data.iter().copied().fold(f64::INFINITY, f64::min);
    let max = data.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !min.is_finite() || !max.is_finite() {
        return None;
    }
    let range = (max - min).max(1e-12);
    let height = min + params.height_frac.clamp(0.0, 1.0) * range;
    let prominence = params.prom_frac.clamp(0.0, 1.0) * range;
    let fs_eff = fs.unwrap_or(1.0).max(1e-6);
    let distance = ((params.min_interval_sec.max(0.0) * fs_eff).round() as usize).max(1);
    Some((height, prominence, distance, range, min, max))
}

/// Peak detection on a 1D series using explicit [`PeakDetectParams`].
///
/// Height/prominence scale with the series range; min spacing uses `fs` when known.
/// Works for raw PPG/resp and for derivative series after process menu.
pub fn detect_peaks_with_params(
    data: &[f64],
    fs: Option<f64>,
    params: &PeakDetectParams,
) -> Vec<usize> {
    let Some((height, prominence, distance, _, _, _)) = peak_thresholds(data, fs, params) else {
        return vec![];
    };

    PeakFinderBuilder::from_slice(data)
        .height(height)
        .prominence(prominence)
        .distance(distance)
        .find()
        .into_iter()
        .map(|p| p.index)
        .collect()
}

/// Run detection into `app.loaded_signal` using `app.peak_params`; update status.
///
/// Also rebuilds the peak–peak tachogram from the active tachogram source.
/// Call after loading, after process apply, or when peak params change.
pub fn run_peak_detection(app: &mut App) -> String {
    let Some(signal) = app.loaded_signal.as_mut() else {
        return "No signal loaded — generate (Ctrl+G) or load a file first.".to_string();
    };
    let peaks = detect_peaks_with_params(&signal.current, signal.fs, &app.peak_params);
    let n_det = peaks.len();
    let n_known = signal.known_peaks_primary.len();
    let tol = app.peak_params.match_tol;
    let matches = if n_known > 0 {
        count_peak_matches(&signal.known_peaks_primary, &peaks, tol)
    } else {
        0
    };
    signal.detected_peaks = peaks;
    signal.show_detected_peaks = true;
    // Detect always updates the tachogram from detected peaks (o can switch to known).
    signal.tachogram_source = crate::app::TachogramSource::Detected;
    signal.rebuild_tachogram();
    let (n_ibi, mean_ibi) = match signal.tachogram.as_ref() {
        Some(t) => {
            let mean = t
                .mean_interval()
                .map(|m| {
                    if t.unit_is_sec {
                        format!("mean IBI={:.3}s", m)
                    } else {
                        format!("mean IBI={:.1} samples", m)
                    }
                })
                .unwrap_or_else(|| "no IBI".into());
            (t.n_intervals(), mean)
        }
        None => (0, "no IBI".into()),
    };

    let thr = peak_thresholds(&signal.current, signal.fs, &app.peak_params);
    let thr_s = thr
        .map(|(h, p, d, _, _, _)| format!("h={:.3} prom={:.3} dist={}smp", h, p, d))
        .unwrap_or_else(|| "n/a".into());

    if n_known > 0 {
        format!(
            "Peaks: {} det | known {} matched {} ±{} | tachogram {} intervals ({})  [{}]  (i view  e export)",
            n_det, n_known, matches, tol, n_ibi, mean_ibi, thr_s
        )
    } else {
        format!(
            "Peaks: {} det | tachogram {} intervals ({})  [{}]  (i view  e export)",
            n_det, n_ibi, mean_ibi, thr_s
        )
    }
}

/// Rebuild tachogram only (e.g. after switching source) without re-detecting peaks.
pub fn rebuild_tachogram_status(app: &mut App) -> String {
    let Some(signal) = app.loaded_signal.as_mut() else {
        return "No signal loaded.".to_string();
    };
    signal.rebuild_tachogram();
    match &signal.tachogram {
        None => format!(
            "Tachogram: need ≥2 {} peaks (k detect or known from generate).",
            signal.tachogram_source.label()
        ),
        Some(t) => {
            let unit = if t.unit_is_sec { "s" } else { "samples" };
            let mean = t
                .mean_interval()
                .map(|m| format!("{:.3}{}", m, unit))
                .unwrap_or_else(|| "n/a".into());
            let rate = t
                .mean_rate()
                .map(|r| format!("{:.1}/min", r))
                .unwrap_or_else(|| "n/a".into());
            format!(
                "Tachogram ({}): {} intervals  mean IBI {}  mean rate {}  (i view  e export  o source)",
                t.source.label(),
                t.n_intervals(),
                mean,
                rate
            )
        }
    }
}

/// Export peak–peak tachogram CSV under `data/`.
///
/// Columns: `interval_index,peak_start_idx,peak_end_idx,peak_end_time,interval,rate_per_min`
pub fn export_tachogram(app: &App) -> anyhow::Result<std::path::PathBuf> {
    use std::io::Write;

    let signal = app
        .loaded_signal
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no signal loaded"))?;
    let tacho = signal
        .tachogram
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no tachogram — run peak detect (k) first"))?;
    if tacho.intervals.is_empty() {
        anyhow::bail!("tachogram has no intervals (need ≥2 peaks)");
    }

    std::fs::create_dir_all("data")?;
    let stem = std::path::Path::new(&signal.name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("signal");
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = std::path::PathBuf::from(format!(
        "data/tachogram_{}_{}_{}.csv",
        stem,
        tacho.source.label().replace(' ', "_"),
        ts
    ));

    let mut f = std::fs::File::create(&path)?;
    let time_hdr = if tacho.unit_is_sec {
        "peak_end_time_sec"
    } else {
        "peak_end_time_samples"
    };
    let ibi_hdr = if tacho.unit_is_sec {
        "interval_sec"
    } else {
        "interval_samples"
    };
    writeln!(
        f,
        "# source={} kind={} fs={} unit={}",
        tacho.source.label(),
        signal.kind.label(),
        signal
            .fs
            .map(|x| format!("{:.6}", x))
            .unwrap_or_else(|| "unknown".into()),
        if tacho.unit_is_sec { "sec" } else { "samples" }
    )?;
    writeln!(
        f,
        "interval_index,peak_start_idx,peak_end_idx,{},{},rate_per_min",
        time_hdr, ibi_hdr
    )?;
    for (i, &ibi) in tacho.intervals.iter().enumerate() {
        let i0 = tacho.peak_indices[i];
        let i1 = tacho.peak_indices[i + 1];
        let t_end = tacho.peak_times[i + 1];
        let rate = tacho.rates_per_min.get(i).copied().unwrap_or(f64::NAN);
        writeln!(
            f,
            "{},{},{},{:.6},{:.6},{:.6}",
            i, i0, i1, t_end, ibi, rate
        )?;
    }
    Ok(path)
}

/// Count known primary peaks matched by a detected peak within `tol` samples.
pub fn count_peak_matches(known: &[usize], detected: &[usize], tol: usize) -> usize {
    known
        .iter()
        .filter(|&&k| detected.iter().any(|&d| d.abs_diff(k) <= tol))
        .count()
}

pub fn generate_demo_and_load(app: &mut App, preset: generate::DemoPreset) -> anyhow::Result<()> {
    let data_dir = std::path::Path::new("data");
    let demo = generate::generate_and_save(preset, data_dir)?;

    if demo.series.is_empty() {
        anyhow::bail!("no numeric data in generated file");
    }

    let n_known = demo.known_peaks_primary.len();
    let n_sec = demo.known_peaks_secondary.len();
    app.loaded_signal = Some(LoadedSignal::with_meta(
        demo.series,
        demo.path.display().to_string(),
        demo.fs,
        demo.kind,
        demo.known_peaks_primary,
        demo.known_peaks_secondary,
    ));
    // Seed tachogram from generator ground-truth so `i` works before first detect.
    if let Some(sig) = app.loaded_signal.as_mut() {
        if n_known >= 2 {
            sig.tachogram_source = crate::app::TachogramSource::KnownPrimary;
            sig.rebuild_tachogram();
        }
    }
    app.peak_params = PeakDetectParams::for_kind(demo.kind);
    app.peak_param_selection = 0;
    app.pending_peak_params = false;
    app.explore_scroll = 0;
    app.explore_view = crate::app::ExploreView::Waveform;
    app.current_tab = Tab::Explore;
    app.current_workflow = crate::app::Workflow::BioSym;
    app.status = format!(
        "Generated {} → {} samples ({})  known peaks: {}/{}  [k detect  i tachogram  e export]",
        demo.path.display(),
        app.loaded_signal.as_ref().map(|s| s.n_samples).unwrap_or(0),
        demo.kind.label(),
        n_known,
        n_sec,
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
