//! Personal SQLite catalog helpers (optional `sqlite` feature).
//!
//! The database file must live outside the SymWorx source tree (typically
//! `$VELOFIT_HOME/db/loadsym.sqlite`). This module never embeds credentials,
//! emails, or other personal identifiers.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use symworx_io::{load_activity, ActivityData};
use symworx_loadsym_db::{get_schema, SCHEMA_VERSION, DEFAULT_DB_RELATIVE};

use crate::load::compute_ride_metrics;

/// Resolve catalog path: `SYMLOAD_DB`, else `$VELOFIT_HOME/db/loadsym.sqlite`, else `~/velofit/db/loadsym.sqlite`.
pub fn default_catalog_path() -> PathBuf {
    if let Ok(p) = std::env::var("SYMLOAD_DB") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    let root = if let Ok(v) = std::env::var("VELOFIT_HOME") {
        if !v.is_empty() {
            PathBuf::from(v)
        } else {
            default_home_velofit()
        }
    } else {
        default_home_velofit()
    };
    root.join(DEFAULT_DB_RELATIVE)
}

fn default_home_velofit() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join("velofit")
}

/// Create parent dirs, open SQLite, apply base schema, run migrations to current version.
pub fn init_catalog(db_path: &Path) -> Result<(), String> {
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create db dir: {}", e))?;
    }
    let conn = Connection::open(db_path).map_err(|e| format!("open sqlite: {}", e))?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| format!("pragma: {}", e))?;
    let schema = get_schema("sqlite");
    conn.execute_batch(schema)
        .map_err(|e| format!("apply schema: {}", e))?;
    migrate_catalog(&conn)?;
    Ok(())
}

/// Open an existing catalog and apply pending migrations.
pub fn open_catalog(db_path: &Path) -> Result<Connection, String> {
    if !db_path.exists() {
        return Err(format!(
            "database not found at {} — run: symload db init",
            db_path.display()
        ));
    }
    let conn = Connection::open(db_path).map_err(|e| format!("open sqlite: {}", e))?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| format!("pragma: {}", e))?;
    // Base CREATE IF NOT EXISTS for any missing tables, then versioned upgrades.
    let schema = get_schema("sqlite");
    let _ = conn.execute_batch(schema);
    migrate_catalog(&conn)?;
    Ok(conn)
}

fn schema_version(conn: &Connection) -> Result<i32, String> {
    let v: Result<i32, _> =
        conn.query_row("SELECT COALESCE(MAX(version), 0) FROM schema_migrations", [], |r| {
            r.get(0)
        });
    match v {
        Ok(n) => Ok(n),
        Err(_) => Ok(0),
    }
}

fn set_schema_version(conn: &Connection, version: i32) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO schema_migrations (version, applied_at) VALUES (?1, datetime('now'))",
        params![version],
    )
    .map_err(|e| format!("record migration: {}", e))?;
    Ok(())
}

/// Apply schema upgrades up to [`SCHEMA_VERSION`].
pub fn migrate_catalog(conn: &Connection) -> Result<(), String> {
    let ver = schema_version(conn)?;
    if ver < 2 {
        migrate_v2_ftp_history(conn)?;
        set_schema_version(conn, 2)?;
    }
    let ver = schema_version(conn)?;
    if ver < SCHEMA_VERSION {
        set_schema_version(conn, SCHEMA_VERSION)?;
    }
    Ok(())
}

fn table_has_column(conn: &Connection, table: &str, col: &str) -> Result<bool, String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({})", table))
        .map_err(|e| e.to_string())?;
    let cols = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(cols.iter().any(|c| c == col))
}

fn migrate_v2_ftp_history(conn: &Connection) -> Result<(), String> {
    // activities.ftp_history_id
    if !table_has_column(conn, "activities", "ftp_history_id")? {
        conn.execute(
            "ALTER TABLE activities ADD COLUMN ftp_history_id INTEGER",
            [],
        )
        .map_err(|e| format!("add ftp_history_id: {}", e))?;
    }

    let has_id = table_has_column(conn, "ftp_history", "id").unwrap_or(false);
    if !has_id {
        // Rebuild from v1 (effective_from PK, ftp_w only) or create empty v2.
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS ftp_history_new (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                effective_from  TEXT NOT NULL,
                effective_to    TEXT,
                ftp_w           REAL NOT NULL CHECK (ftp_w > 0),
                sport           TEXT NOT NULL DEFAULT 'cycling',
                source          TEXT,
                notes           TEXT,
                created_at      TEXT DEFAULT (datetime('now')),
                UNIQUE (sport, effective_from)
            );
            ",
        )
        .map_err(|e| format!("create ftp_history_new: {}", e))?;

        // Copy if old table exists with expected columns
        let old_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='ftp_history'",
                [],
                |r| r.get::<_, i64>(0).map(|n| n > 0),
            )
            .unwrap_or(false);

        if old_exists && table_has_column(conn, "ftp_history", "ftp_w")? {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO ftp_history_new (effective_from, ftp_w, sport, source)
                 SELECT effective_from, ftp_w, 'cycling', 'migrated_v1' FROM ftp_history",
                [],
            );
            conn.execute_batch(
                "
                DROP TABLE ftp_history;
                ALTER TABLE ftp_history_new RENAME TO ftp_history;
                ",
            )
            .map_err(|e| format!("swap ftp_history: {}", e))?;
        } else {
            conn.execute_batch(
                "
                DROP TABLE IF EXISTS ftp_history;
                ALTER TABLE ftp_history_new RENAME TO ftp_history;
                ",
            )
            .map_err(|e| format!("install ftp_history: {}", e))?;
        }
    } else {
        // Ensure optional columns on already-v2-ish tables
        if !table_has_column(conn, "ftp_history", "effective_to")? {
            let _ = conn.execute("ALTER TABLE ftp_history ADD COLUMN effective_to TEXT", []);
        }
        if !table_has_column(conn, "ftp_history", "sport")? {
            let _ = conn.execute(
                "ALTER TABLE ftp_history ADD COLUMN sport TEXT NOT NULL DEFAULT 'cycling'",
                [],
            );
        }
        if !table_has_column(conn, "ftp_history", "source")? {
            let _ = conn.execute("ALTER TABLE ftp_history ADD COLUMN source TEXT", []);
        }
        if !table_has_column(conn, "ftp_history", "notes")? {
            let _ = conn.execute("ALTER TABLE ftp_history ADD COLUMN notes TEXT", []);
        }
        if !table_has_column(conn, "ftp_history", "created_at")? {
            let _ = conn.execute(
                "ALTER TABLE ftp_history ADD COLUMN created_at TEXT DEFAULT (datetime('now'))",
                [],
            );
        }
    }

    conn.execute_batch(
        "
        CREATE INDEX IF NOT EXISTS idx_ftp_history_lookup ON ftp_history (sport, effective_from);
        CREATE INDEX IF NOT EXISTS idx_activities_ftp_history ON activities (ftp_history_id);
        ",
    )
    .map_err(|e| format!("ftp indexes: {}", e))?;

    Ok(())
}

/// Resolved FTP for scoring a ride.
#[derive(Debug, Clone)]
pub struct FtpResolution {
    pub ftp_w: f64,
    pub ftp_history_id: Option<i64>,
    /// `history` | `fallback`
    pub origin: &'static str,
}

/// Look up FTP for a calendar date from `ftp_history`, else use `fallback_ftp`.
///
/// Sport defaults to `cycling` when `sport` is None/empty.
pub fn resolve_ftp(
    conn: &Connection,
    ride_date: &str,
    sport: Option<&str>,
    fallback_ftp: f64,
) -> Result<FtpResolution, String> {
    let sport = sport
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("cycling");

    let row = conn
        .query_row(
            "SELECT id, ftp_w FROM ftp_history
             WHERE sport = ?1
               AND effective_from <= ?2
               AND (effective_to IS NULL OR effective_to > ?2)
             ORDER BY effective_from DESC
             LIMIT 1",
            params![sport, ride_date],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    if let Some((id, ftp_w)) = row {
        return Ok(FtpResolution {
            ftp_w,
            ftp_history_id: Some(id),
            origin: "history",
        });
    }

    // Also try sport-agnostic: any row matching date if sport-specific miss
    // (no-op if only cycling rows exist)

    Ok(FtpResolution {
        ftp_w: fallback_ftp.max(50.0),
        ftp_history_id: None,
        origin: "fallback",
    })
}

/// Insert or replace an FTP history row.
pub fn set_ftp_history(
    conn: &Connection,
    effective_from: &str,
    ftp_w: f64,
    sport: &str,
    source: Option<&str>,
    notes: Option<&str>,
    effective_to: Option<&str>,
) -> Result<i64, String> {
    if ftp_w <= 0.0 {
        return Err("ftp_w must be > 0".into());
    }
    let sport = if sport.is_empty() { "cycling" } else { sport };
    conn.execute(
        "INSERT INTO ftp_history (effective_from, effective_to, ftp_w, sport, source, notes, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))
         ON CONFLICT(sport, effective_from) DO UPDATE SET
           effective_to=excluded.effective_to,
           ftp_w=excluded.ftp_w,
           source=excluded.source,
           notes=excluded.notes",
        params![
            effective_from,
            effective_to,
            ftp_w,
            sport,
            source,
            notes
        ],
    )
    .map_err(|e| e.to_string())?;
    let id: i64 = conn
        .query_row(
            "SELECT id FROM ftp_history WHERE sport = ?1 AND effective_from = ?2",
            params![sport, effective_from],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(id)
}

/// List FTP history rows (newest first).
pub fn list_ftp_history(conn: &Connection) -> Result<Vec<(i64, String, Option<String>, f64, String, Option<String>)>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, effective_from, effective_to, ftp_w, sport, source
             FROM ftp_history ORDER BY effective_from DESC, sport",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// Stable source key: path relative to archive root when possible, else file name only.
/// Avoids embedding absolute home-directory paths in the catalog.
pub fn source_key(path: &Path, archive_root: Option<&Path>) -> String {
    if let Some(root) = archive_root {
        if let Ok(rel) = path.strip_prefix(root) {
            return rel.to_string_lossy().replace('\\', "/");
        }
    }
    path.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// SHA-256 hex of file contents.
pub fn file_sha256(path: &Path) -> Result<String, String> {
    let mut f = File::open(path).map_err(|e| format!("open for hash: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf).map_err(|e| format!("read for hash: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Best-effort ride date (`YYYY-MM-DD`).
///
/// Priority:
/// 1. Date embedded in filename (`2015-05-05-…` / `2017_07_06_…`)
/// 2. FIT/activity internal start timestamp (when `act` provided)
/// 3. Filesystem mtime — **last resort only** (wrong after rclone/S3 copy bulk mtimes)
pub fn infer_ride_date(path: &Path) -> String {
    infer_ride_date_with_activity(path, None)
}

/// Like [`infer_ride_date`] but prefers timestamps from an already-loaded activity.
pub fn infer_ride_date_with_activity(path: &Path, act: Option<&ActivityData>) -> String {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if let Some(d) = parse_date_from_name(name) {
            // Guard: only accept plausible sports years
            if date_year_plausible(&d) {
                return d;
            }
        }
    }
    if let Some(act) = act {
        if let Some(d) = act.start_date_ymd() {
            if date_year_plausible(&d) {
                return d;
            }
        }
    }
    // Last resort: mtime (often "day of sync" for archives — avoid for bulk re-date)
    if let Ok(meta) = fs::metadata(path) {
        if let Ok(mtime) = meta.modified() {
            if let Ok(dur) = mtime.duration_since(std::time::UNIX_EPOCH) {
                let days = dur.as_secs() / 86400;
                if let Some(d) = unix_days_to_ymd(days) {
                    if date_year_plausible(&d) {
                        return d;
                    }
                }
            }
        }
    }
    "1970-01-01".to_string()
}

fn date_year_plausible(ymd: &str) -> bool {
    // YYYY-MM-DD
    if ymd.len() < 4 {
        return false;
    }
    let y: i32 = ymd[..4].parse().unwrap_or(0);
    (1990..=2100).contains(&y)
}

fn parse_date_from_name(name: &str) -> Option<String> {
    // 2015-05-05-15-05-24.fit  or  2017_07_06_12_42_23.fit
    let re_dash = regex_lite_date_dash(name);
    if re_dash.is_some() {
        return re_dash;
    }
    regex_lite_date_uscore(name)
}

fn regex_lite_date_dash(name: &str) -> Option<String> {
    // Find YYYY-MM-DD
    let bytes = name.as_bytes();
    for i in 0..bytes.len().saturating_sub(9) {
        if bytes[i].is_ascii_digit()
            && bytes[i + 1].is_ascii_digit()
            && bytes[i + 2].is_ascii_digit()
            && bytes[i + 3].is_ascii_digit()
            && bytes[i + 4] == b'-'
            && bytes[i + 5].is_ascii_digit()
            && bytes[i + 6].is_ascii_digit()
            && bytes[i + 7] == b'-'
            && bytes[i + 8].is_ascii_digit()
            && bytes[i + 9].is_ascii_digit()
        {
            return Some(name[i..i + 10].to_string());
        }
    }
    None
}

fn regex_lite_date_uscore(name: &str) -> Option<String> {
    // YYYY_MM_DD
    let bytes = name.as_bytes();
    for i in 0..bytes.len().saturating_sub(9) {
        if bytes[i].is_ascii_digit()
            && bytes[i + 1].is_ascii_digit()
            && bytes[i + 2].is_ascii_digit()
            && bytes[i + 3].is_ascii_digit()
            && bytes[i + 4] == b'_'
            && bytes[i + 5].is_ascii_digit()
            && bytes[i + 6].is_ascii_digit()
            && bytes[i + 7] == b'_'
            && bytes[i + 8].is_ascii_digit()
            && bytes[i + 9].is_ascii_digit()
        {
            let y = &name[i..i + 4];
            let m = &name[i + 5..i + 7];
            let d = &name[i + 8..i + 10];
            return Some(format!("{}-{}-{}", y, m, d));
        }
    }
    None
}

fn unix_days_to_ymd(days: u64) -> Option<String> {
    // Civil from days algorithm (Howard Hinnant), UTC
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    if y < 0 || y > 9999 {
        return None;
    }
    Some(format!("{:04}-{:02}-{:02}", y, m, d))
}

/// Result of attempting to ingest one activity file.
#[derive(Debug, Clone)]
pub enum IngestOutcome {
    Inserted {
        source_key: String,
        tss: f64,
        ftp_w: f64,
        ftp_origin: &'static str,
    },
    Skipped {
        source_key: String,
        reason: String,
    },
    Failed {
        path: String,
        error: String,
    },
}

/// Ingest one `.fit` (or activity) file into the catalog.
///
/// FTP is resolved from `ftp_history` for the ride date when present; otherwise
/// `fallback_ftp` is used. Pass `force = true` to re-score even if `file_hash` exists.
pub fn ingest_one(
    conn: &Connection,
    path: &Path,
    fallback_ftp: f64,
    archive_root: Option<&Path>,
    force: bool,
) -> IngestOutcome {
    let key = source_key(path, archive_root);
    let hash = match file_sha256(path) {
        Ok(h) => h,
        Err(e) => {
            return IngestOutcome::Failed {
                path: path.display().to_string(),
                error: e,
            };
        }
    };

    if !force {
        match conn
            .query_row(
                "SELECT id FROM activities WHERE file_hash = ?1 LIMIT 1",
                params![&hash],
                |row| row.get::<_, i64>(0),
            )
            .optional()
        {
            Ok(Some(_)) => {
                return IngestOutcome::Skipped {
                    source_key: key,
                    reason: "file_hash already present (use --force to re-score)".into(),
                };
            }
            Ok(None) => {}
            Err(e) => {
                return IngestOutcome::Failed {
                    path: path.display().to_string(),
                    error: e.to_string(),
                };
            }
        }
    }

    let act = match load_activity(&path.to_string_lossy()) {
        Ok(a) => a,
        Err(e) => {
            return IngestOutcome::Failed {
                path: path.display().to_string(),
                error: e.to_string(),
            };
        }
    };

    if act.is_empty() {
        return IngestOutcome::Skipped {
            source_key: key,
            reason: "no samples".into(),
        };
    }

    // Prefer FIT internal start time over mtime (mtime is often "day of S3 copy").
    let ride_date = infer_ride_date_with_activity(path, Some(&act));
    let sport = act.sport.as_deref();
    let ftp_res = match resolve_ftp(conn, &ride_date, sport, fallback_ftp) {
        Ok(r) => r,
        Err(e) => {
            return IngestOutcome::Failed {
                path: path.display().to_string(),
                error: e,
            };
        }
    };

    let power = act.power_series();
    let metrics = compute_ride_metrics(&act.times_s, &power, ftp_res.ftp_w);
    let n = act.len() as f64;
    let avg_p = if n > 0.0 {
        power.iter().sum::<f64>() / n
    } else {
        0.0
    };
    let max_p = power.iter().copied().fold(0.0_f64, f64::max);
    let (avg_hr, max_hr) = series_avg_max_opt(&act.heart_rate_bpm);
    let (avg_cad, max_cad) = series_avg_max_opt(&act.cadence);
    let (avg_spd, max_spd) = {
        let (a, m) = series_avg_max_opt(&act.speed_mps);
        (a.map(|v| v * 3.6), m.map(|v| v * 3.6))
    };

    let file_size = fs::metadata(path).map(|m| m.len() as i64).unwrap_or(0);
    let platform = guess_platform(&act);

    let res = conn.execute(
        "INSERT INTO activities (
            source_file, file_hash, ride_date, duration_s, sport,
            manufacturer, product, source_platform,
            avg_power_w, max_power_w, np_w, tss, intensity_factor, ftp_used_w, ftp_history_id, total_work_kj,
            avg_hr_bpm, max_hr_bpm, avg_cadence, max_cadence, avg_speed_kmh, max_speed_kmh,
            file_size, imported_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5,
            ?6, ?7, ?8,
            ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
            ?17, ?18, ?19, ?20, ?21, ?22,
            ?23, datetime('now')
        )
        ON CONFLICT(source_file) DO UPDATE SET
            file_hash=excluded.file_hash,
            ride_date=excluded.ride_date,
            duration_s=excluded.duration_s,
            np_w=excluded.np_w,
            tss=excluded.tss,
            intensity_factor=excluded.intensity_factor,
            ftp_used_w=excluded.ftp_used_w,
            ftp_history_id=excluded.ftp_history_id,
            avg_power_w=excluded.avg_power_w,
            max_power_w=excluded.max_power_w,
            total_work_kj=excluded.total_work_kj,
            imported_at=datetime('now')
        ",
        params![
            key,
            hash,
            ride_date,
            act.duration_s(),
            act.sport,
            act.manufacturer,
            act.product,
            platform,
            avg_p,
            max_p,
            metrics.np,
            metrics.tss,
            metrics.if_,
            ftp_res.ftp_w,
            ftp_res.ftp_history_id,
            metrics.total_work_kj,
            avg_hr,
            max_hr,
            avg_cad,
            max_cad,
            avg_spd,
            max_spd,
            file_size,
        ],
    );

    match res {
        Ok(_) => {
            if let Err(e) = recompute_daily_for_date(conn, &ride_date) {
                return IngestOutcome::Failed {
                    path: path.display().to_string(),
                    error: format!("daily rollup: {}", e),
                };
            }
            IngestOutcome::Inserted {
                source_key: key,
                tss: metrics.tss,
                ftp_w: ftp_res.ftp_w,
                ftp_origin: ftp_res.origin,
            }
        }
        Err(e) => IngestOutcome::Failed {
            path: path.display().to_string(),
            error: e.to_string(),
        },
    }
}

fn series_avg_max_opt(v: &[Option<f64>]) -> (Option<f64>, Option<f64>) {
    let mut sum = 0.0;
    let mut n = 0usize;
    let mut max = f64::NEG_INFINITY;
    for x in v {
        if let Some(val) = x {
            if val.is_finite() {
                sum += val;
                n += 1;
                if *val > max {
                    max = *val;
                }
            }
        }
    }
    if n == 0 {
        (None, None)
    } else {
        (Some(sum / n as f64), Some(max))
    }
}

fn guess_platform(act: &ActivityData) -> Option<String> {
    let m = act.manufacturer.as_deref()?.to_ascii_lowercase();
    if m.contains("garmin") {
        Some("garmin".into())
    } else if m.contains("srm") {
        Some("srm".into())
    } else if m.contains("polar") {
        Some("polar".into())
    } else if m.contains("wahoo") {
        Some("wahoo".into())
    } else {
        Some(m)
    }
}

/// Rebuild *all* `daily_loads` rows from `activities` (clears stale dates like bulk-mtime piles).
pub fn recompute_all_daily_loads(conn: &Connection) -> Result<usize, String> {
    conn.execute("DELETE FROM daily_loads", [])
        .map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT DISTINCT ride_date FROM activities ORDER BY ride_date")
        .map_err(|e| e.to_string())?;
    let dates: Vec<String> = stmt
        .query_map([], |r| r.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    for d in &dates {
        recompute_daily_for_date(conn, d)?;
    }
    Ok(dates.len())
}

/// Recompute daily_loads row for one date from activities.
pub fn recompute_daily_for_date(conn: &Connection, ride_date: &str) -> Result<(), String> {
    let mut stmt = conn
        .prepare(
            "SELECT COALESCE(SUM(tss),0), COALESCE(SUM(duration_s),0), COUNT(*),
                    (SELECT sport FROM activities WHERE ride_date = ?1 AND sport IS NOT NULL LIMIT 1)
             FROM activities WHERE ride_date = ?1",
        )
        .map_err(|e| e.to_string())?;
    let (tss, dur, count, sport): (f64, f64, i64, Option<String>) = stmt
        .query_row(params![ride_date], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(|e| e.to_string())?;

    if count == 0 {
        conn.execute("DELETE FROM daily_loads WHERE ride_date = ?1", params![ride_date])
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    conn.execute(
        "INSERT INTO daily_loads (ride_date, total_tss, total_duration_s, ride_count, primary_sport, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
         ON CONFLICT(ride_date) DO UPDATE SET
           total_tss=excluded.total_tss,
           total_duration_s=excluded.total_duration_s,
           ride_count=excluded.ride_count,
           primary_sport=excluded.primary_sport,
           updated_at=datetime('now')",
        params![ride_date, tss, dur, count, sport],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Recompute ACWR-style load_metrics for all days with daily_loads (chronological).
pub fn recompute_load_metrics(conn: &Connection) -> Result<usize, String> {
    use crate::load::{classify_acwr, compute_acute_chronic, compute_monotony, compute_strain};

    let mut stmt = conn
        .prepare("SELECT ride_date, total_tss FROM daily_loads ORDER BY ride_date ASC")
        .map_err(|e| e.to_string())?;
    let rows: Vec<(String, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let loads: Vec<f64> = rows.iter().map(|(_, t)| *t).collect();
    let mut n_written = 0usize;

    for i in 0..rows.len() {
        let date = &rows[i].0;
        let prefix = &loads[..=i];
        let (acute, chronic, acwr, risk) = match compute_acute_chronic(prefix, 7, 28) {
            Ok(s) => (
                Some(s.acute_load),
                Some(s.chronic_load),
                Some(s.acwr),
                Some(classify_acwr(s.acwr).as_str().to_string()),
            ),
            Err(_) => (None, None, None, None),
        };
        let mono = compute_monotony(prefix).ok();
        let strain = compute_strain(prefix).ok();

        conn.execute(
            "INSERT INTO load_metrics (
                ride_date, acute_load, chronic_load, acwr, risk_level,
                monotony, strain, computed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'))
             ON CONFLICT(ride_date) DO UPDATE SET
                acute_load=excluded.acute_load,
                chronic_load=excluded.chronic_load,
                acwr=excluded.acwr,
                risk_level=excluded.risk_level,
                monotony=excluded.monotony,
                strain=excluded.strain,
                computed_at=datetime('now')",
            params![date, acute, chronic, acwr, risk, mono, strain],
        )
        .map_err(|e| e.to_string())?;
        n_written += 1;
    }
    Ok(n_written)
}

/// Count activities in the catalog.
pub fn count_activities(conn: &Connection) -> Result<i64, String> {
    conn.query_row("SELECT COUNT(*) FROM activities", [], |r| r.get(0))
        .map_err(|e| e.to_string())
}

/// Per-ride summary for calendar daily lists.
#[derive(Debug, Clone)]
pub struct RideSummary {
    pub ride_date: String,
    pub source_file: String,
    pub tss: f64,
    pub duration_s: f64,
    pub np_w: Option<f64>,
    pub avg_power_w: Option<f64>,
}

/// Load ride rows ordered by date then file (for calendar file lists).
pub fn load_ride_summaries(conn: &Connection) -> Result<Vec<RideSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT ride_date, source_file,
                    COALESCE(tss, 0), COALESCE(duration_s, 0),
                    np_w, avg_power_w
             FROM activities
             ORDER BY ride_date ASC, source_file ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(RideSummary {
                ride_date: row.get(0)?,
                source_file: row.get(1)?,
                tss: row.get(2)?,
                duration_s: row.get(3)?,
                np_w: row.get(4)?,
                avg_power_w: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// One day of load history for calendar / trend views.
#[derive(Debug, Clone)]
pub struct DailySnapshot {
    /// `YYYY-MM-DD`
    pub ride_date: String,
    pub total_tss: f64,
    pub ride_count: i64,
    pub primary_sport: Option<String>,
    /// Precomputed ACWR when present in `load_metrics`.
    pub acwr: Option<f64>,
    pub risk_level: Option<String>,
    pub monotony: Option<f64>,
    pub strain: Option<f64>,
    pub acute_load: Option<f64>,
    pub chronic_load: Option<f64>,
}

/// Load chronological daily series from the catalog (for TUI Calendar).
///
/// Joins `daily_loads` with optional `load_metrics`. Empty if no rows.
pub fn load_daily_snapshots(conn: &Connection) -> Result<Vec<DailySnapshot>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT d.ride_date, d.total_tss, d.ride_count, d.primary_sport,
                    lm.acwr, lm.risk_level, lm.monotony, lm.strain,
                    lm.acute_load, lm.chronic_load
             FROM daily_loads d
             LEFT JOIN load_metrics lm ON lm.ride_date = d.ride_date
             ORDER BY d.ride_date ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(DailySnapshot {
                ride_date: row.get(0)?,
                total_tss: row.get(1)?,
                ride_count: row.get(2)?,
                primary_sport: row.get(3)?,
                acwr: row.get(4)?,
                risk_level: row.get(5)?,
                monotony: row.get(6)?,
                strain: row.get(7)?,
                acute_load: row.get(8)?,
                chronic_load: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// Convenience: open default catalog path and load daily + ride rows for the TUI.
/// Returns `Ok(None)` if the DB file does not exist (not an error for the TUI).
pub fn try_load_default_calendar(
) -> Result<Option<(PathBuf, Vec<DailySnapshot>, Vec<RideSummary>)>, String> {
    let path = default_catalog_path();
    if !path.exists() {
        return Ok(None);
    }
    let conn = open_catalog(&path)?;
    let rows = load_daily_snapshots(&conn)?;
    let rides = load_ride_summaries(&conn)?;
    Ok(Some((path, rows, rides)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_dates() {
        assert_eq!(
            parse_date_from_name("2015-05-05-15-05-24.fit").as_deref(),
            Some("2015-05-05")
        );
        assert_eq!(
            parse_date_from_name("2017_07_06_12_42_23.fit").as_deref(),
            Some("2017-07-06")
        );
        assert_eq!(
            parse_date_from_name("prefix_2012-03-23-14-27-47Z_0.fit").as_deref(),
            Some("2012-03-23")
        );
    }

    #[test]
    fn source_key_strips_root() {
        let root = Path::new("/home/user/velofit");
        let p = Path::new("/home/user/velofit/raw/ride.fit");
        assert_eq!(source_key(p, Some(root)), "raw/ride.fit");
        assert_eq!(source_key(p, None), "ride.fit");
    }
}
