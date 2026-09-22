// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! SQLite backend. The file is opened read-only and `query_only` is set.

use std::path::Path;

use rusqlite::{
    Connection,
    OpenFlags,
    OptionalExtension,
    params,
    types::ValueRef,
};

use crate::{
    error::Result,
    model::{
        Column,
        Page,
        Relation,
        format_hint,
        sqlite_page_sql,
    },
    quote::quote_ident,
    source::CatalogSource,
};

/// Open SQLite file.
pub struct SqliteSource {
    label: String,
    conn: Connection,
}

impl SqliteSource {
    /// Open `path` read-only. Missing files and directories are errors.
    pub fn open(path: &Path) -> Result<Self> {
        if path.is_dir() {
            return Err(crate::error::Error::InvalidParameter(format!(
                "path is a directory: {}",
                path.display()
            )));
        }
        if !path.exists() {
            return Err(crate::error::Error::InvalidParameter(format!(
                "database not found: {}",
                path.display()
            )));
        }
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        conn.execute_batch("PRAGMA query_only = ON;")?;
        Ok(Self {
            label: path.display().to_string(),
            conn,
        })
    }

    fn has_table(&self, name: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [name],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}

impl CatalogSource for SqliteSource {
    fn label(&self) -> &str {
        &self.label
    }

    fn relations(&mut self) -> Result<Vec<Relation>> {
        let mut stmt = self.conn.prepare(
            "SELECT name FROM sqlite_master
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
             ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut relations = Vec::new();
        for name in rows {
            relations.push(Relation {
                schema: "main".to_string(),
                name: name?,
            });
        }
        Ok(relations)
    }

    fn columns(&mut self, relation: &Relation) -> Result<Vec<Column>> {
        let sql = pragma_table_info(relation);
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            let name: String = row.get(1)?;
            let type_name: String = row.get(2)?;
            let not_null: i64 = row.get(3)?;
            Ok(Column {
                name,
                type_name,
                nullable: not_null == 0,
            })
        })?;
        let mut columns = Vec::new();
        for column in rows {
            columns.push(column?);
        }
        Ok(columns)
    }

    fn page(&mut self, relation: &Relation, limit: u32, offset: u32) -> Result<Page> {
        let columns = self.columns(relation)?;
        if columns.is_empty() {
            return Ok(Page {
                columns,
                rows: Vec::new(),
                offset,
                limit,
            });
        }
        let sql = sqlite_page_sql(relation, &columns);
        let mut stmt = self.conn.prepare(&sql)?;
        let width = columns.len();
        let mapped = stmt.query_map(params![i64::from(limit), i64::from(offset)], |row| {
            let mut cells = Vec::with_capacity(width);
            for index in 0..width {
                cells.push(cell_from_value(row.get_ref(index)?));
            }
            Ok(cells)
        })?;
        let mut rows = Vec::new();
        for row in mapped {
            rows.push(row?);
        }
        Ok(Page {
            columns,
            rows,
            offset,
            limit,
        })
    }

    fn catalog_hint(&mut self) -> Result<Option<String>> {
        let profile = if self.has_table("catalog_meta")? {
            self.conn
                .query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'profile' LIMIT 1",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
        } else {
            None
        };
        let version = if self.has_table("schema_migrations")? {
            self.conn
                .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                    row.get::<_, Option<i64>>(0)
                })
                .optional()?
                .flatten()
        } else {
            None
        };
        Ok(format_hint(profile.as_deref(), version))
    }
}

fn pragma_table_info(relation: &Relation) -> String {
    let table = quote_ident(&relation.name);
    if relation.schema.is_empty() || relation.schema == "main" {
        format!("PRAGMA table_info({table})")
    } else {
        format!("PRAGMA {}.table_info({table})", quote_ident(&relation.schema))
    }
}

fn cell_from_value(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Null => String::new(),
        ValueRef::Integer(value) => value.to_string(),
        ValueRef::Real(value) => value.to_string(),
        ValueRef::Text(bytes) => String::from_utf8_lossy(bytes).into_owned(),
        ValueRef::Blob(bytes) => format!("<blob {} bytes>", bytes.len()),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{
            AtomicU64,
            Ordering,
        },
        time::{
            SystemTime,
            UNIX_EPOCH,
        },
    };

    use rusqlite::Connection;
    use symworx_dbsym::SQLITE_SCHEMA;

    use super::{
        SqliteSource,
        cell_from_value,
        pragma_table_info,
    };
    use crate::{
        model::Relation,
        source::CatalogSource,
    };

    #[test]
    fn pragma_quotes_the_table_name() {
        let relation = Relation {
            schema: "main".to_string(),
            name: "subjects".to_string(),
        };
        assert_eq!(pragma_table_info(&relation), "PRAGMA table_info(\"subjects\")");
    }

    #[test]
    fn blob_cells_are_a_length_marker() {
        let blob = rusqlite::types::ValueRef::Blob(&[1, 2, 3]);
        assert_eq!(cell_from_value(blob), "<blob 3 bytes>");
        assert_eq!(cell_from_value(rusqlite::types::ValueRef::Null), "");
    }

    #[test]
    fn browses_template_catalog_and_rejects_writes() {
        let file = TempDb::new();
        {
            let conn = Connection::open(&file.0).unwrap();
            conn.execute_batch(SQLITE_SCHEMA).unwrap();
            conn.execute(
                "INSERT INTO subjects (id, coded_id) VALUES (?1, ?2)",
                ["11111111-1111-1111-1111-111111111111", "S001"],
            )
            .unwrap();
            conn.execute("INSERT INTO catalog_meta (key, value) VALUES ('profile', 'study')", [])
                .unwrap();
            conn.execute("INSERT INTO schema_migrations (version) VALUES (1)", [])
                .unwrap();
        }

        let mut source = SqliteSource::open(&file.0).unwrap();
        let names: Vec<String> = source.relations().unwrap().into_iter().map(|rel| rel.name).collect();
        assert!(names.contains(&"subjects".to_string()));
        assert!(names.contains(&"sessions".to_string()));

        let subjects = Relation {
            schema: "main".to_string(),
            name: "subjects".to_string(),
        };
        let columns = source.columns(&subjects).unwrap();
        let column_names: Vec<&str> = columns.iter().map(|column| column.name.as_str()).collect();
        assert_eq!(
            column_names,
            [
                "id",
                "combined_hash",
                "coded_id",
                "created_at",
                "ingest_batch_id",
                "status",
                "reason"
            ]
        );

        let page = source.page(&subjects, 100, 0).unwrap();
        let coded = page
            .columns
            .iter()
            .position(|column| column.name == "coded_id")
            .unwrap();
        assert_eq!(page.rows.len(), 1);
        assert_eq!(page.rows[0][coded], "S001");
        assert_eq!(
            source.catalog_hint().unwrap().as_deref(),
            Some("profile=study · schema v1")
        );

        let write = source.conn.execute("INSERT INTO subjects (id) VALUES ('x')", []);
        assert!(write.is_err());
    }

    struct TempDb(PathBuf);

    impl TempDb {
        fn new() -> Self {
            static N: AtomicU64 = AtomicU64::new(0);
            let n = N.fetch_add(1, Ordering::Relaxed);
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let path = std::env::temp_dir().join(format!("symdb-view-{}-{}-{n}.sqlite", std::process::id(), nanos));
            Self(path)
        }
    }

    impl Drop for TempDb {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
}
