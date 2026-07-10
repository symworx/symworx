// symworx-loadsym-db
//
// Canonical SQL for bootstrapping a personal training-load catalog.
// Zero runtime dependencies. No sample data, no personal identifiers.
//
// Apply the schema yourself (or via `symload db init`) against a DB file
// that lives *outside* this repository (e.g. $VELOFIT_HOME/db/loadsym.sqlite).

/// Schema version for migrations / `schema_migrations` table.
///
/// v1 — initial catalog  
/// v2 — expanded `ftp_history` + `activities.ftp_history_id` for time-varying FTP scoring
pub const SCHEMA_VERSION: i32 = 2;

/// PostgreSQL schema (shared / multi-user deployments).
pub const POSTGRES_SCHEMA: &str = include_str!("../sql/schema.sql");

/// SQLite schema (recommended for single-user local catalogs).
pub const SQLITE_SCHEMA: &str = include_str!("../sql/schema.sqlite.sql");

/// Returns the schema for the requested dialect.
///
/// Accepts: `postgres` / `pg` / `postgresql`, `sqlite` / `sql`.
/// Unknown values default to SQLite (personal default).
pub fn get_schema(dialect: &str) -> &'static str {
    match dialect.to_ascii_lowercase().as_str() {
        "postgres" | "pg" | "postgresql" => POSTGRES_SCHEMA,
        "sqlite" | "sql" | "" => SQLITE_SCHEMA,
        _ => SQLITE_SCHEMA,
    }
}

/// Default relative path (under `$VELOFIT_HOME`) for the personal SQLite catalog.
/// Callers must resolve this against their own archive root — never hardcode usernames.
pub const DEFAULT_DB_RELATIVE: &str = "db/loadsym.sqlite";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schemas_are_not_empty() {
        assert!(POSTGRES_SCHEMA.len() > 100);
        assert!(SQLITE_SCHEMA.len() > 100);
        assert!(SCHEMA_VERSION >= 1);
        assert!(SQLITE_SCHEMA.contains("CREATE TABLE IF NOT EXISTS activities"));
        assert!(SQLITE_SCHEMA.contains("INTEGER PRIMARY KEY AUTOINCREMENT"));
    }

    #[test]
    fn default_dialect_is_sqlite() {
        assert!(get_schema("sqlite").contains("PRAGMA foreign_keys"));
        assert!(get_schema("unknown").contains("PRAGMA foreign_keys"));
        assert!(get_schema("postgres").contains("BIGSERIAL"));
    }
}
