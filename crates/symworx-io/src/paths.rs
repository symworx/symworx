// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Personal archive path helpers (velofit) and activity file discovery.
//!
//! These are thin filesystem utilities used by `symload` and the TUI.
//! They intentionally have no network or DB dependencies.

use std::{
    path::{
        Path,
        PathBuf,
    },
    time::SystemTime,
};

/// Environment variable that overrides the default `~/velofit` root.
pub const VELOFIT_HOME_ENV: &str = "VELOFIT_HOME";

/// Default personal ride archive root: `$VELOFIT_HOME` or `$HOME/velofit`.
pub fn default_velofit_root() -> PathBuf {
    if let Ok(v) = std::env::var(VELOFIT_HOME_ENV) {
        let p = PathBuf::from(v);
        if !p.as_os_str().is_empty() {
            return p;
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join("velofit")
}

/// Default email / manual drop zone: `~/velofit/inbox`.
pub fn default_velofit_inbox() -> PathBuf {
    default_velofit_root().join("inbox")
}

/// Default raw archive (S3-mirrored): `~/velofit/raw`.
pub fn default_velofit_raw() -> PathBuf {
    default_velofit_root().join("raw")
}

/// Polar AccessLink FIT drop zone: `$VELOFIT_HOME/raw/polar`.
pub fn default_velofit_polar() -> PathBuf {
    default_velofit_raw().join("polar")
}

/// Default SQLite catalog path: `$VELOFIT_HOME/db/loadsym.sqlite`.
///
/// Override with `SYMLOAD_DB` when set. The data file is personal — never commit it.
pub fn default_velofit_db() -> PathBuf {
    if let Ok(p) = std::env::var("SYMLOAD_DB")
        && !p.is_empty()
    {
        return PathBuf::from(p);
    }
    default_velofit_root().join("db").join("loadsym.sqlite")
}

/// Candidate directories for LoadSym / activity discovery (in priority order).
///
/// Includes velofit inbox/raw when they exist or always as preferred roots,
/// plus common project-relative folders.
pub fn default_activity_search_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        default_velofit_inbox(),
        default_velofit_raw(),
        PathBuf::from("data"),
        PathBuf::from("rides"),
        PathBuf::from("training"),
    ];
    // De-dup while preserving order
    let mut seen = std::collections::HashSet::new();
    dirs.retain(|d| seen.insert(d.clone()));
    dirs
}

/// File metadata useful for picking the newest activity without parsing FIT.
#[derive(Debug, Clone)]
pub struct ActivityFileEntry {
    /// Absolute or as-provided path.
    pub path: PathBuf,
    /// Modification time when available.
    pub modified: Option<SystemTime>,
}

/// Discover `.fit` / `.csv` / `.txt` activity files under the given directories.
///
/// When `recursive` is false, only the top level of each directory is scanned
/// (matches the current flat `~/velofit/raw/` layout). Results are sorted by
/// modification time descending (newest first).
pub fn discover_activity_files(dirs: &[PathBuf], recursive: bool) -> Vec<ActivityFileEntry> {
    let mut out = Vec::new();
    for dir in dirs {
        if !dir.exists() {
            continue;
        }
        if recursive {
            walk_recursive(dir, &mut out);
        } else {
            scan_one_dir(dir, &mut out);
        }
    }
    out.sort_by(|a, b| match (a.modified, b.modified) {
        (Some(am), Some(bm)) => bm.cmp(&am),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => b.path.cmp(&a.path),
    });
    out
}

/// Convenience: newest activity path under default search dirs (non-recursive).
pub fn find_newest_activity_path() -> Option<PathBuf> {
    let dirs = default_activity_search_dirs();
    discover_activity_files(&dirs, false).into_iter().next().map(|e| e.path)
}

fn scan_one_dir(dir: &Path, out: &mut Vec<ActivityFileEntry>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if !p.is_file() {
            continue;
        }
        if !is_activity_ext(&p) {
            continue;
        }
        let modified = e.metadata().ok().and_then(|m| m.modified().ok());
        out.push(ActivityFileEntry { path: p, modified });
    }
}

fn walk_recursive(dir: &Path, out: &mut Vec<ActivityFileEntry>) {
    scan_one_dir(dir, out);
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            // Skip hidden / scratch
            if let Some(name) = p.file_name().and_then(|n| n.to_str())
                && name.starts_with('.')
            {
                continue;
            }
            walk_recursive(&p, out);
        }
    }
}

fn is_activity_ext(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| {
            let el = ext.to_lowercase();
            matches!(el.as_str(), "fit" | "csv" | "txt")
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::Write,
    };

    use super::*;

    #[test]
    fn discover_newest_fit() {
        let tmp = std::env::temp_dir().join(format!("symworx_io_paths_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let older = tmp.join("older.fit");
        let newer = tmp.join("newer.fit");
        {
            let mut f = fs::File::create(&older).unwrap();
            f.write_all(b"old").unwrap();
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
        {
            let mut f = fs::File::create(&newer).unwrap();
            f.write_all(b"new").unwrap();
        }
        let found = discover_activity_files(&[tmp.clone()], false);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].path, newer);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn velofit_paths_are_under_root() {
        let root = default_velofit_root();
        assert!(default_velofit_inbox().starts_with(&root));
        assert!(default_velofit_raw().starts_with(&root));
        assert_eq!(default_velofit_inbox().file_name().unwrap(), "inbox");
        assert_eq!(default_velofit_raw().file_name().unwrap(), "raw");
    }
}
