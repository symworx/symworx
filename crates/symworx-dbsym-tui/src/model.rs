// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Relation, column, and page values shared by both drivers.

use crate::quote::quote_ident;

/// Rows fetched per page.
pub const PAGE_SIZE: u32 = 100;

/// A table the browser can open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relation {
    /// `main` / `public`, or another schema name.
    pub schema: String,
    /// Table name.
    pub name: String,
}

impl Relation {
    /// Label in the relation list. Unqualified for `main` and `public`.
    pub fn label(&self) -> String {
        if unqualified(&self.schema) {
            self.name.clone()
        } else {
            format!("{}.{}", self.schema, self.name)
        }
    }

    /// Quoted `schema.table` for a `FROM` clause.
    pub fn qualified_sql(&self) -> String {
        if unqualified(&self.schema) {
            quote_ident(&self.name)
        } else {
            format!("{}.{}", quote_ident(&self.schema), quote_ident(&self.name))
        }
    }
}

fn unqualified(schema: &str) -> bool {
    schema.is_empty() || schema == "main" || schema == "public"
}

/// One column of a relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    /// Column name.
    pub name: String,
    /// Declared type (`TEXT`, `integer`, …). Empty when the driver has none.
    pub type_name: String,
    /// True when the column accepts NULL.
    pub nullable: bool,
}

/// One window of rows. Cells are display strings. NULL is an empty string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// Columns in select order.
    pub columns: Vec<Column>,
    /// Row-major cells. Each row matches `columns`.
    pub rows: Vec<Vec<String>>,
    /// Zero-based offset of `rows[0]` in the table.
    pub offset: u32,
    /// Requested page size.
    pub limit: u32,
}

/// Footer fragment when `catalog_meta` / `schema_migrations` exist.
pub fn format_hint(profile: Option<&str>, version: Option<i64>) -> Option<String> {
    match (profile, version) {
        (Some(profile), Some(version)) => Some(format!("profile={profile} · schema v{version}")),
        (Some(profile), None) => Some(format!("profile={profile}")),
        (None, Some(version)) => Some(format!("schema v{version}")),
        (None, None) => None,
    }
}

/// Shorten `s` to `width` characters, ending with an ellipsis when cut.
pub fn truncate_chars(s: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let count = s.chars().count();
    if count <= width {
        return s.to_string();
    }
    if width == 1 {
        return "…".to_string();
    }
    let mut out: String = s.chars().take(width - 1).collect();
    out.push('…');
    out
}

/// `SELECT` list and `LIMIT` for SQLite. Caller guarantees `columns` is non-empty.
pub fn sqlite_page_sql(relation: &Relation, columns: &[Column]) -> String {
    let cols = columns
        .iter()
        .map(|column| quote_ident(&column.name))
        .collect::<Vec<_>>()
        .join(", ");
    format!("SELECT {cols} FROM {} LIMIT ?1 OFFSET ?2", relation.qualified_sql())
}

/// `SELECT col::text` and `LIMIT` for Postgres. Caller guarantees `columns` is non-empty.
pub fn postgres_page_sql(relation: &Relation, columns: &[Column]) -> String {
    let cols = columns
        .iter()
        .map(|column| format!("{}::text", quote_ident(&column.name)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("SELECT {cols} FROM {} LIMIT $1 OFFSET $2", relation.qualified_sql())
}

#[cfg(test)]
mod tests {
    use super::{
        Column,
        Relation,
        format_hint,
        postgres_page_sql,
        sqlite_page_sql,
        truncate_chars,
    };

    fn id_column() -> Column {
        Column {
            name: "id".to_string(),
            type_name: "TEXT".to_string(),
            nullable: false,
        }
    }

    #[test]
    fn public_and_main_stay_unqualified() {
        let relation = Relation {
            schema: "public".to_string(),
            name: "subjects".to_string(),
        };
        assert_eq!(relation.label(), "subjects");
        assert_eq!(relation.qualified_sql(), "\"subjects\"");
    }

    #[test]
    fn other_schemas_are_qualified() {
        let relation = Relation {
            schema: "study".to_string(),
            name: "subjects".to_string(),
        };
        assert_eq!(relation.label(), "study.subjects");
        assert_eq!(relation.qualified_sql(), "\"study\".\"subjects\"");
    }

    #[test]
    fn page_sql_quotes_injected_names() {
        let relation = Relation {
            schema: "public".to_string(),
            name: "subjects\"; drop".to_string(),
        };
        let columns = [id_column()];
        let sqlite = sqlite_page_sql(&relation, &columns);
        let postgres = postgres_page_sql(&relation, &columns);
        assert!(sqlite.contains("\"subjects\"\"; drop\""));
        assert!(sqlite.contains("LIMIT ?1 OFFSET ?2"));
        assert!(postgres.contains("\"id\"::text"));
        assert!(postgres.contains("\"subjects\"\"; drop\""));
        assert!(postgres.contains("LIMIT $1 OFFSET $2"));
    }

    #[test]
    fn hint_joins_profile_and_version() {
        assert_eq!(
            format_hint(Some("study"), Some(1)).as_deref(),
            Some("profile=study · schema v1")
        );
        assert_eq!(format_hint(None, None), None);
    }

    #[test]
    fn truncate_keeps_short_strings_and_marks_the_cut() {
        assert_eq!(truncate_chars("id", 8), "id");
        assert_eq!(truncate_chars("abcdef", 4), "abc…");
    }
}
