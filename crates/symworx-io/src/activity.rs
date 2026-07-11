// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Sport-agnostic exercise / activity file I/O (FIT primary, CSV fallback for power).
//!
//! Per plan: core primitives for power, speed, etc. series used by LoadSym.
//! FIT support gated behind `fit` feature to keep default footprint small.
//!
//! Default output location for TUI-generated/imported data is `data/` (or user supplied).

#[allow(unused_imports)]
use std::fs::File;

use symworx_error::SymError;

/// Rich activity data loaded from FIT (or CSV).
/// Supports data from different devices (Garmin, Polar, SRM, etc.).
/// All vectors are same length. Missing values are None.
#[derive(Debug, Clone, Default)]
pub struct ActivityData {
    /// Source filename or device label
    pub source: String,
    /// Device manufacturer from FIT file_id (e.g. "garmin", "srm").
    pub manufacturer: Option<String>,
    /// Device product name/id from FIT file_id.
    pub product: Option<String>,
    /// Sport type string (e.g. "cycling", "running")
    pub sport: Option<String>,

    /// Relative time in seconds (0-based)
    pub times_s: Vec<f64>,
    /// Power in watts (common for powermeters: SRM, Garmin, etc.)
    pub power_w: Vec<Option<f64>>,
    /// Heart rate (bpm) — available from most HR monitors (Polar, Garmin, etc.)
    pub heart_rate_bpm: Vec<Option<f64>>,
    /// Speed in m/s
    pub speed_mps: Vec<Option<f64>>,
    /// Cadence (rpm or spm)
    pub cadence: Vec<Option<f64>>,
    // Future: position_lat/long (semicircles), altitude, etc.
}

impl ActivityData {
    /// Total duration in seconds (last relative time or count-based fallback).
    pub fn duration_s(&self) -> f64 {
        self.times_s
            .last()
            .copied()
            .unwrap_or(self.times_s.len() as f64)
            .max(0.0)
    }

    /// Number of samples.
    pub fn len(&self) -> usize {
        self.times_s.len()
    }

    /// True if no samples.
    pub fn is_empty(&self) -> bool {
        self.times_s.is_empty()
    }

    /// Return a clean power series with missing → 0.0
    pub fn power_series(&self) -> Vec<f64> {
        self.power_w.iter().map(|v| v.unwrap_or(0.0)).collect()
    }

    /// Return whether this activity has any non-zero power samples.
    pub fn has_power(&self) -> bool {
        self.power_w.iter().any(|v| v.is_some_and(|p| p > 0.0))
    }
}

/// Read a full ActivityData from a FIT file.
///
/// Collects power, hr, speed, cadence when present.
/// Extracts basic file metadata (manufacturer/product) from file_id if available.
/// Times are made relative (starting at 0).
#[cfg(feature = "fit")]
pub fn load_fit_activity(path: &str) -> Result<ActivityData, SymError> {
    use std::io::BufReader;

    use fitparser::{
        FitDataRecord,
        Value,
    };

    let file = File::open(path).map_err(SymError::Io)?;
    let mut reader = BufReader::new(file);

    let records: Vec<FitDataRecord> = fitparser::from_reader(&mut reader)
        .map_err(|e| SymError::UnsupportedFormat(format!("FIT parse error: {}", e)))?;

    let mut manufacturer = None;
    let mut product = None;
    let mut sport = None;

    let mut times_raw: Vec<f64> = vec![];
    let mut power: Vec<Option<f64>> = vec![];
    let mut hr: Vec<Option<f64>> = vec![];
    let mut speed: Vec<Option<f64>> = vec![];
    let mut cadence: Vec<Option<f64>> = vec![];

    let mut last_ts: Option<f64> = None;

    for rec in records {
        let kind = rec.kind();

        if kind == fitparser::profile::MesgNum::FileId {
            for f in rec.fields() {
                match f.name() {
                    "manufacturer" => {
                        manufacturer = Some(fit_value_display(f.value()));
                    }
                    "product" => {
                        product = Some(fit_value_display(f.value()));
                    }
                    _ => {}
                }
            }
            continue;
        }

        if kind == fitparser::profile::MesgNum::Sport {
            for f in rec.fields() {
                if f.name() == "sport" {
                    sport = Some(fit_value_display(f.value()));
                }
            }
        }

        if kind == fitparser::profile::MesgNum::Record {
            let mut p: Option<f64> = None;
            let mut h: Option<f64> = None;
            let mut s: Option<f64> = None;
            let mut c: Option<f64> = None;
            let mut ts: Option<f64> = None;

            for field in rec.fields() {
                match field.name() {
                    "timestamp" => {
                        // fitparser sometimes exposes as u32 or Timestamp variant
                        if let Value::UInt32(v) = field.value() {
                            ts = Some(*v as f64);
                        }
                    }
                    "power" => match field.value() {
                        Value::UInt16(v) => p = Some(*v as f64),
                        Value::Float64(v) => p = Some(*v),
                        _ => {}
                    },
                    "heart_rate" => {
                        if let Value::UInt8(v) = field.value() {
                            h = Some(*v as f64);
                        }
                    }
                    "speed" => {
                        if let Value::UInt16(v) = field.value() {
                            // Common scaling in FIT: speed is m/s * 1000
                            s = Some(*v as f64 / 1000.0);
                        } else if let Value::Float64(v) = field.value() {
                            s = Some(*v);
                        }
                    }
                    "cadence" => {
                        if let Value::UInt8(v) = field.value() {
                            c = Some(*v as f64);
                        }
                    }
                    _ => {}
                }
            }

            // Build relative time from FIT timestamps.
            // FIT timestamps are absolute (u32 seconds since ~1989-12-31 epoch).
            // We compute elapsed by successive deltas when present; otherwise fall back to 1s steps.
            let t = if let Some(tval) = ts {
                if let Some(prev_abs) = last_ts {
                    // tval is expected larger; use delta
                    let dt = (tval - prev_abs).max(0.0);
                    // accumulate a synthetic relative from previous relative + dt
                    // (we store relative in times_raw after first)
                    times_raw.last().copied().unwrap_or(0.0) + dt
                } else {
                    0.0
                }
            } else {
                // no timestamp on this record: 1 s step
                times_raw.last().copied().unwrap_or(0.0) + 1.0
            };

            // Track last *absolute* ts for delta computation on next record
            if let Some(tval) = ts {
                last_ts = Some(tval);
            }
            times_raw.push(t);
            power.push(p);
            hr.push(h);
            speed.push(s);
            cadence.push(c);
        }
    }

    let n = times_raw.len();
    if n == 0 {
        return Err(SymError::UnsupportedFormat(
            "no record messages found in FIT".into(),
        ));
    }

    // Make times strictly relative starting at 0
    let t0 = times_raw[0];
    let times_s: Vec<f64> = times_raw.into_iter().map(|t| (t - t0).max(0.0)).collect();

    Ok(ActivityData {
        source: path.to_string(),
        manufacturer,
        product,
        sport,
        times_s,
        power_w: power,
        heart_rate_bpm: hr,
        speed_mps: speed,
        cadence,
    })
}

/// Fallback / non-fit: try to read a simple power CSV (time,power or just power).
/// Useful for testing without real FITs.
pub fn read_power_csv_series(path: &str) -> Result<(Vec<f64>, Vec<f64>), SymError> {
    // Reuse the csv reader logic but pick last two columns heuristically.
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .map_err(|e| SymError::UnsupportedFormat(e.to_string()))?;

    let mut times = Vec::new();
    let mut powers = Vec::new();
    let mut t = 0.0;

    for result in rdr.records() {
        let record = result.map_err(|e| SymError::UnsupportedFormat(format!("csv row: {}", e)))?;
        // Try last column as power; second last as optional time or ignore
        if let Some(last) = record.get(record.len() - 1)
            && let Ok(p) = last.trim().parse::<f64>()
        {
            powers.push(p);
            times.push(t);
            t += 1.0;
        }
    }

    if powers.is_empty() {
        // try no-header numeric last-col
        // fall back to simpler path
        use crate::traits::SymReader;
        return crate::csv::CsvReader::read(path)
            .map(|rows| {
                let p: Vec<f64> = rows.iter().filter_map(|r| r.last().copied()).collect();
                let tt: Vec<f64> = (0..p.len()).map(|i| i as f64).collect();
                (tt, p)
            })
            .map_err(|_| SymError::UnsupportedFormat("no usable power data in csv".into()));
    }
    Ok((times, powers))
}

/// Human-readable string for a fitparser field value (avoids `String("garmin")` Debug form).
#[cfg(feature = "fit")]
fn fit_value_display(v: &fitparser::Value) -> String {
    use fitparser::Value;
    match v {
        Value::String(s) => s.clone(),
        other => {
            let s = format!("{:?}", other);
            // Strip common Debug wrappers: String("x") / "x"
            if let Some(inner) = s
                .strip_prefix("String(\"")
                .and_then(|t| t.strip_suffix("\")"))
            {
                inner.to_string()
            } else if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
                s[1..s.len() - 1].to_string()
            } else {
                s
            }
        }
    }
}

/// Load a full ActivityData (recommended for new code).
/// Supports .fit (with feature) and header-aware CSV.
pub fn load_activity(path: &str) -> Result<ActivityData, SymError> {
    let lower = path.to_lowercase();
    if lower.ends_with(".fit") {
        #[cfg(feature = "fit")]
        return load_fit_activity(path);
        #[cfg(not(feature = "fit"))]
        return Err(SymError::UnsupportedFormat(
            "enable `fit` feature on symworx-io for .fit files".into(),
        ));
    }
    if lower.ends_with(".csv") || lower.ends_with(".txt") {
        return load_activity_from_csv(path);
    }
    Err(SymError::UnsupportedFormat(path.into()))
}

/// Backward compat for old power-only callers.
pub fn load_activity_power_series(path: &str) -> Result<(Vec<f64>, Vec<f64>), SymError> {
    let data = load_activity(path)?;
    let p: Vec<f64> = data.power_w.iter().map(|v| v.unwrap_or(0.0)).collect();
    Ok((data.times_s, p))
}

/// Load generic activity CSV that may have headers like time,power,heart_rate,speed,cadence.
fn load_activity_from_csv(path: &str) -> Result<ActivityData, SymError> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .map_err(|e| SymError::UnsupportedFormat(e.to_string()))?;

    let headers = rdr
        .headers()
        .map_err(|e| SymError::UnsupportedFormat(e.to_string()))?
        .clone();

    let find = |name: &str| headers.iter().position(|h| h.eq_ignore_ascii_case(name));

    let col_time = find("time");
    let col_power = find("power");
    let col_hr = find("heart_rate").or_else(|| find("hr"));
    let col_speed = find("speed");
    let col_cad = find("cadence");

    let mut times = Vec::new();
    let mut power = Vec::new();
    let mut hr = Vec::new();
    let mut spd = Vec::new();
    let mut cad = Vec::new();

    let mut t = 0.0;

    for result in rdr.records() {
        let rec = result.map_err(|e| SymError::UnsupportedFormat(format!("csv: {}", e)))?;

        let get_f64 = |col: Option<usize>| -> Option<f64> {
            col.and_then(|i| rec.get(i))
                .and_then(|s| s.trim().parse::<f64>().ok())
        };

        if let Some(c) = col_time
            && let Some(val) = rec.get(c).and_then(|s| s.parse::<f64>().ok())
        {
            t = val;
        }

        times.push(t);
        power.push(get_f64(col_power));
        hr.push(get_f64(col_hr));
        spd.push(get_f64(col_speed));
        cad.push(get_f64(col_cad));

        if col_time.is_none() {
            t += 1.0;
        }
    }

    if times.is_empty() {
        return Err(SymError::UnsupportedFormat("empty activity csv".into()));
    }

    // normalize times to start at 0
    let t0 = times[0];
    let times_s: Vec<f64> = times.into_iter().map(|v| (v - t0).max(0.0)).collect();

    Ok(ActivityData {
        source: path.to_string(),
        manufacturer: None,
        product: None,
        sport: None,
        times_s,
        power_w: power,
        heart_rate_bpm: hr,
        speed_mps: spd,
        cadence: cad,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_csv_fallback_shape() {
        // The test data dir may not have a power csv, just ensure it doesn't panic on bad path.
        let res = load_activity_power_series("data/demo_stride_intervals.csv");
        // Either succeeds or gives clear error — don't assert content
        let _ = res;
    }

    #[test]
    fn load_user_srm_garmin_style_ride() {
        // User's real ride file copied for testing (see user query)
        // Note: when testing the io *crate* the cwd is crates/symworx-io, so we go up to workspace root data/
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let path = format!("{}/../../data/2026_07_05_ride.fit", manifest);
        match load_activity(&path) {
            Ok(act) => {
                let n = act.len();
                let dur = act.duration_s();
                let has_p = act.has_power();
                let p = act.power_series();
                let avg_p = if n > 0 {
                    p.iter().sum::<f64>() / n as f64
                } else {
                    0.0
                };
                let max_p = p.iter().copied().fold(0.0_f64, f64::max);

                eprintln!("\n=== Loaded real ride from user ===");
                eprintln!("source: {}", act.source);
                eprintln!("manufacturer: {:?}", act.manufacturer);
                eprintln!("product: {:?}", act.product);
                eprintln!("sport: {:?}", act.sport);
                eprintln!(
                    "samples: {}   duration: {:.1} s   (~{:.1} min)",
                    n,
                    dur,
                    dur / 60.0
                );
                eprintln!(
                    "has_power: {}   avg_power: {:.1} W   max_power: {:.0} W",
                    has_p, avg_p, max_p
                );

                // Inline minimal NP/TSS for inspection (same math as loadsym)
                // (avoids pulling heavy linalg via loadsym in this crate's test)
                let ftp = 280.0f64;
                let np = if n >= 30 {
                    let mut fourth = 0.0;
                    let mut cnt = 0usize;
                    for w in p.windows(30) {
                        let m30 = w.iter().sum::<f64>() / 30.0;
                        fourth += m30.powi(4);
                        cnt += 1;
                    }
                    if cnt > 0 {
                        (fourth / cnt as f64).powf(0.25)
                    } else {
                        avg_p
                    }
                } else {
                    avg_p
                };
                let if_ = if ftp > 0.0 { np / ftp } else { 0.0 };
                let tss = (dur * np * if_) / (ftp * 36.0);
                let work_kj = avg_p * dur / 1000.0;

                eprintln!("--- Computed (FTP guess {:.0} W) ---", ftp);
                eprintln!("NP: {:.0} W   IF: {:.2}   TSS: {:.1}", np, if_, tss);
                eprintln!("work_kj ≈ {:.0}", work_kj);

                let m2 = {
                    let ftp2 = 320.0;
                    let if2 = np / ftp2;
                    let tss2 = (dur * np * if2) / (ftp2 * 36.0);
                    (if2, tss2)
                };
                eprintln!("(for FTP=320 W -> IF: {:.2}  TSS: {:.1})", m2.0, m2.1);
            }
            Err(e) => {
                eprintln!("Could not load user's ride {}: {}", path, e);
            }
        }
    }
}
