// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Per-study (or edge) research catalog: subjects, sessions, attributes, and file provenance.
//!
//! The crate ships a **template** schema. `symdb init` copies it to
//! `<project>/.dbsym/schema.sqlite.sql` for the study to edit and commit.
//! The SQLite file itself stays gitignored. Signal bytes still go through `symworx-io`.
//!
//! This is **not** the LoadSym personal ride catalog (`symworx-loadsym-db`).

#![doc(html_root_url = "https://docs.rs/symworx-dbsym")]

pub mod error;

#[cfg(feature = "sqlite")]
pub mod init;
#[cfg(feature = "sqlite")]
pub mod seed;

pub use error::{
    DbSymError,
    Result,
};

/// Schema version for `schema_migrations`.
///
/// v1 — subjects, sessions, ingest batches, EAV observations, file records
pub const SCHEMA_VERSION: i32 = 1;

/// SQLite **template** shipped in this crate. Copied into the project on init;
/// afterwards the study file is the source of truth.
pub const SQLITE_SCHEMA: &str = include_str!("../sql/schema.sqlite.sql");

/// Default relative path under the project root for the SQLite file (gitignored).
pub const DEFAULT_DB_RELATIVE: &str = ".dbsym/dbsym.sqlite";

/// Study-owned schema SQL (commit this; init will not overwrite an existing file).
pub const DEFAULT_SCHEMA_RELATIVE: &str = ".dbsym/schema.sqlite.sql";

/// Relative object directory next to the SQLite file.
pub const DEFAULT_OBJECTS_RELATIVE: &str = ".dbsym/objects";

/// Install profile. Same DDL; different typical contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreProfile {
    /// Multi-subject study directory.
    Study,
    /// One-subject capture node (phone / Pi).
    Node,
}

impl StoreProfile {
    /// Stable catalog_meta value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Study => "study",
            Self::Node => "node",
        }
    }

    /// Parse `study` / `node` (case-insensitive). Unknown → `None`.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "study" => Some(Self::Study),
            "node" => Some(Self::Node),
            _ => None,
        }
    }
}

/// Returns the schema for the requested dialect.
///
/// Accepts: `sqlite` / `sql` / empty. Unknown values default to SQLite.
/// Postgres is not shipped in v1 (same dual-SQL pattern as `symworx-loadsym-db` later).
pub fn get_schema(dialect: &str) -> &'static str {
    match dialect.to_ascii_lowercase().as_str() {
        "postgres" | "pg" | "postgresql" => SQLITE_SCHEMA,
        "sqlite" | "sql" | "" => SQLITE_SCHEMA,
        _ => SQLITE_SCHEMA,
    }
}

/// Current version of the `symworx-dbsym` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_is_not_empty() {
        assert!(SQLITE_SCHEMA.len() > 100);
        assert!(SCHEMA_VERSION >= 1);
        assert!(SQLITE_SCHEMA.contains("CREATE TABLE IF NOT EXISTS subjects"));
        assert!(SQLITE_SCHEMA.contains("CREATE TABLE IF NOT EXISTS sessions"));
        assert!(SQLITE_SCHEMA.contains("CREATE TABLE IF NOT EXISTS file_records"));
        assert!(SQLITE_SCHEMA.contains("CREATE TABLE IF NOT EXISTS observations"));
        assert!(SQLITE_SCHEMA.contains("CREATE TABLE IF NOT EXISTS observation_values"));
        assert!(SQLITE_SCHEMA.contains("CREATE TABLE IF NOT EXISTS catalog_meta"));
        assert!(SQLITE_SCHEMA.contains("INTEGER PRIMARY KEY AUTOINCREMENT"));
        assert!(!SQLITE_SCHEMA.to_ascii_lowercase().contains("insert into subjects"));
    }

    #[test]
    fn default_dialect_is_sqlite() {
        assert!(get_schema("sqlite").contains("PRAGMA foreign_keys"));
        assert!(get_schema("unknown").contains("PRAGMA foreign_keys"));
        assert!(get_schema("").contains("PRAGMA foreign_keys"));
    }

    #[test]
    fn default_paths_are_under_dbsym() {
        assert_eq!(DEFAULT_DB_RELATIVE, ".dbsym/dbsym.sqlite");
        assert_eq!(DEFAULT_SCHEMA_RELATIVE, ".dbsym/schema.sqlite.sql");
        assert!(DEFAULT_OBJECTS_RELATIVE.starts_with(".dbsym/"));
    }

    #[test]
    fn store_profile_roundtrip() {
        assert_eq!(StoreProfile::parse("study"), Some(StoreProfile::Study));
        assert_eq!(StoreProfile::parse("NODE"), Some(StoreProfile::Node));
        assert_eq!(StoreProfile::parse("lab"), None);
        assert_eq!(StoreProfile::Study.as_str(), "study");
        assert_eq!(StoreProfile::Node.as_str(), "node");
    }
}
