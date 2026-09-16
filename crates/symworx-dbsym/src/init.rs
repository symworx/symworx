// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Create a per-project (or edge) SQLite store. Mirrors `symworx-loadsym::catalog::init_catalog`.

use std::{
    fs,
    path::{
        Path,
        PathBuf,
    },
};

use rusqlite::Connection;

use crate::{
    DEFAULT_DB_RELATIVE,
    DEFAULT_OBJECTS_RELATIVE,
    DEFAULT_SCHEMA_RELATIVE,
    SCHEMA_VERSION,
    SQLITE_SCHEMA,
    StoreProfile,
    error::Result,
    seed::apply_presets,
};

const GITIGNORE: &str = "*.sqlite\nobjects/\n";

/// What `init` created.
#[derive(Debug, Clone)]
pub struct InitReport {
    /// Project / node root.
    pub root: PathBuf,
    /// SQLite path.
    pub db: PathBuf,
    /// Object directory.
    pub objects: PathBuf,
    /// Study-owned schema SQL (copied from the crate template on first init).
    pub schema: PathBuf,
    /// Attributes inserted from presets.
    pub attributes: usize,
    /// Install profile.
    pub profile: StoreProfile,
}

/// Create directories, apply schema, record profile, seed presets.
pub fn init(root: &Path, profile: StoreProfile, presets: &[String]) -> Result<InitReport> {
    let root = if root.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        root.to_path_buf()
    };
    fs::create_dir_all(&root)?;

    let db = root.join(DEFAULT_DB_RELATIVE);
    if let Some(parent) = db.parent() {
        fs::create_dir_all(parent)?;
        let gi = parent.join(".gitignore");
        if !gi.exists() {
            fs::write(&gi, GITIGNORE)?;
        }
    }
    let objects = root.join(DEFAULT_OBJECTS_RELATIVE);
    fs::create_dir_all(&objects)?;

    let schema = root.join(DEFAULT_SCHEMA_RELATIVE);
    install_schema_template(&schema)?;

    let conn = Connection::open(&db)?;
    apply_schema_file(&conn, &schema)?;
    conn.execute(
        "INSERT OR IGNORE INTO schema_migrations (version) VALUES (?1)",
        rusqlite::params![SCHEMA_VERSION],
    )?;
    upsert_meta(&conn, "profile", profile.as_str())?;
    let project = root.file_name().and_then(|s| s.to_str()).unwrap_or("project");
    upsert_meta(&conn, "project", project)?;
    upsert_meta(&conn, "template_version", &SCHEMA_VERSION.to_string())?;

    let attributes = apply_presets(&conn, presets)?;

    Ok(InitReport {
        root,
        db,
        objects,
        schema,
        attributes,
        profile,
    })
}

/// Copy the crate template into `schema` only if that file does not exist.
pub fn install_schema_template(schema: &Path) -> Result<()> {
    if schema.exists() {
        return Ok(());
    }
    if let Some(parent) = schema.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(schema, SQLITE_SCHEMA)?;
    Ok(())
}

/// SQL beside the database (`<dir>/.dbsym/schema.sqlite.sql`), else the crate template.
pub fn schema_sql_for_db(db_path: &Path) -> Result<String> {
    let beside = db_path
        .parent()
        .map(|p| p.join("schema.sqlite.sql"))
        .filter(|p| p.is_file());
    match beside {
        Some(p) => Ok(fs::read_to_string(p)?),
        None => Ok(SQLITE_SCHEMA.to_string()),
    }
}

/// Apply a schema SQL file (`CREATE IF NOT EXISTS` is idempotent).
pub fn apply_schema_file(conn: &Connection, schema: &Path) -> Result<()> {
    let sql = fs::read_to_string(schema)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.execute_batch(&sql)?;
    Ok(())
}

/// Re-apply the study schema next to an existing database (after local SQL edits).
pub fn apply(root: &Path) -> Result<PathBuf> {
    let db = root.join(DEFAULT_DB_RELATIVE);
    let schema = root.join(DEFAULT_SCHEMA_RELATIVE);
    if !schema.is_file() {
        return Err(crate::error::DbSymError::InvalidParameter(format!(
            "no study schema at {} — run: symdb init",
            schema.display()
        )));
    }
    let conn = open(&db)?;
    apply_schema_file(&conn, &schema)?;
    Ok(schema)
}

/// Open an existing catalog (must already exist).
///
/// Applies the study schema file beside the DB when present so local DDL edits
/// take effect without touching the crate.
pub fn open(db_path: &Path) -> Result<Connection> {
    if !db_path.exists() {
        return Err(crate::error::DbSymError::InvalidParameter(format!(
            "database not found at {} — run: symdb init",
            db_path.display()
        )));
    }
    let conn = Connection::open(db_path)?;
    let sql = schema_sql_for_db(db_path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.execute_batch(&sql)?;
    Ok(conn)
}

fn upsert_meta(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO catalog_meta (key, value, updated_at) VALUES (?1, ?2, datetime('now'))
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
        rusqlite::params![key, value],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed::available_presets;

    fn scratch() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "symworx-dbsym-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn init_study_seeds_exercise_science() {
        let root = scratch();
        let report = init(&root, StoreProfile::Study, &["exercise_science".to_string()]).expect("init");
        assert!(report.db.exists());
        assert!(report.objects.is_dir());
        assert!(report.attributes >= 1);
        assert!(available_presets().contains(&"exercise_science"));

        let conn = open(&report.db).expect("open");
        let profile: String = conn
            .query_row("SELECT value FROM catalog_meta WHERE key = 'profile'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(profile, "study");
        let n: i32 = conn
            .query_row("SELECT COUNT(*) FROM attributes", [], |r| r.get(0))
            .unwrap();
        assert!(n >= 1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn init_node_blank_has_no_attributes() {
        let root = scratch();
        let report = init(&root, StoreProfile::Node, &[]).expect("init");
        let conn = open(&report.db).expect("open");
        let n: i32 = conn
            .query_row("SELECT COUNT(*) FROM attributes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
        let profile: String = conn
            .query_row("SELECT value FROM catalog_meta WHERE key = 'profile'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(profile, "node");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn unknown_preset_fails() {
        let root = scratch();
        let err = init(&root, StoreProfile::Study, &["not_a_preset".to_string()]).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unknown preset"), "{msg}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn init_copies_schema_and_does_not_overwrite() {
        let root = scratch();
        let report = init(&root, StoreProfile::Study, &[]).expect("init");
        assert!(report.schema.is_file());
        assert!(
            fs::read_to_string(&report.schema)
                .unwrap()
                .contains("CREATE TABLE IF NOT EXISTS subjects")
        );
        fs::write(
            &report.schema,
            "-- study owned\nCREATE TABLE IF NOT EXISTS subjects (id TEXT PRIMARY KEY);\nCREATE TABLE IF NOT EXISTS study_extra (id INTEGER PRIMARY KEY);\n",
        )
        .unwrap();
        init(&root, StoreProfile::Study, &[]).expect("re-init");
        let sql = fs::read_to_string(&report.schema).unwrap();
        assert!(sql.contains("study_extra"), "init must not clobber study schema");
        apply(&root).expect("apply");
        let conn = open(&report.db).expect("open");
        let n: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'study_extra'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
        let _ = fs::remove_dir_all(&root);
    }
}
