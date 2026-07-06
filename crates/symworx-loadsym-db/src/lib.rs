// symworx-loadsym-db
//
// Provides the canonical SQL for bootstrapping a personal training load database.
// Designed to be used by *separate* user projects (your DB + periodization app)
// and by the `symload` CLI.
//
// The actual application (migrations over time, connection pooling, business logic)
// belongs in your separate repo.
//
// Schemas are versioned. See `SCHEMA_VERSION`.

pub const SCHEMA_VERSION: i32 = 1;

/// PostgreSQL schema (recommended for shared or more advanced use).
pub const POSTGRES_SCHEMA: &str = include_str!("../sql/schema.sql");

/// SQLite variant (for personal single-file use, easy to rclone).
/// Minor differences noted in comments in the main file.
pub const SQLITE_SCHEMA: &str = include_str!("../sql/schema.sql");

/// Returns the schema for the requested dialect.
pub fn get_schema(dialect: &str) -> &'static str {
    match dialect.to_ascii_lowercase().as_str() {
        "postgres" | "pg" | "postgresql" => POSTGRES_SCHEMA,
        "sqlite" | "sql" => SQLITE_SCHEMA,
        _ => POSTGRES_SCHEMA,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schemas_are_not_empty() {
        assert!(POSTGRES_SCHEMA.len() > 100);
        assert!(SQLITE_SCHEMA.len() > 100);
        assert!(SCHEMA_VERSION >= 1);
    }
}
