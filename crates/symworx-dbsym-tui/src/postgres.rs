// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Postgres backend. Connections use `NoTls` and a read-only session.
//! `sslmode=require` (and verify modes) are refused until a TLS feature exists.

use postgres::{
    Client,
    Config,
    NoTls,
    config::{
        Host,
        SslMode,
    },
};

use crate::{
    error::{
        Error,
        Result,
    },
    model::{
        Column,
        Page,
        Relation,
        format_hint,
        postgres_page_sql,
    },
    source::CatalogSource,
};

/// List user tables. Skips the system catalogs.
pub const LIST_RELATIONS_SQL: &str = "\
SELECT table_schema, table_name
FROM information_schema.tables
WHERE table_type = 'BASE TABLE'
  AND table_schema NOT IN ('pg_catalog', 'information_schema')
ORDER BY table_schema, table_name";

/// Columns for one table. `$1` schema, `$2` name.
pub const LIST_COLUMNS_SQL: &str = "\
SELECT column_name, data_type, is_nullable
FROM information_schema.columns
WHERE table_schema = $1 AND table_name = $2
ORDER BY ordinal_position";

/// Open Postgres connection.
pub struct PostgresSource {
    label: String,
    client: Client,
}

impl PostgresSource {
    /// Connect and set `default_transaction_read_only`. Password comes from the URL or `PGPASSWORD`.
    pub fn connect(url: &str) -> Result<Self> {
        if url_requires_tls(url) {
            return Err(Error::TlsRequired(url.to_string()));
        }
        let mut config: Config = url.parse()?;
        config.ssl_mode(SslMode::Disable);
        config.application_name("symdb-view");
        let from_env = std::env::var("PGPASSWORD").ok();
        apply_password(&mut config, from_env.as_deref());
        let label = display_label(&config);
        let mut client = config.connect(NoTls)?;
        client.batch_execute("SET default_transaction_read_only = on")?;
        Ok(Self { label, client })
    }

    fn has_table(&mut self, name: &str) -> Result<bool> {
        let row = self.client.query_one(
            "SELECT COUNT(*)::bigint FROM information_schema.tables
             WHERE table_type = 'BASE TABLE'
               AND table_name = $1
               AND table_schema NOT IN ('pg_catalog', 'information_schema')",
            &[&name],
        )?;
        let count: i64 = row.get(0);
        Ok(count > 0)
    }
}

impl CatalogSource for PostgresSource {
    fn label(&self) -> &str {
        &self.label
    }

    fn relations(&mut self) -> Result<Vec<Relation>> {
        let rows = self.client.query(LIST_RELATIONS_SQL, &[])?;
        let mut relations = Vec::with_capacity(rows.len());
        for row in rows {
            relations.push(Relation {
                schema: row.get(0),
                name: row.get(1),
            });
        }
        Ok(relations)
    }

    fn columns(&mut self, relation: &Relation) -> Result<Vec<Column>> {
        let rows = self
            .client
            .query(LIST_COLUMNS_SQL, &[&relation.schema, &relation.name])?;
        let mut columns = Vec::with_capacity(rows.len());
        for row in rows {
            let nullable: String = row.get(2);
            columns.push(Column {
                name: row.get(0),
                type_name: row.get(1),
                nullable: nullable.eq_ignore_ascii_case("YES"),
            });
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
        let sql = postgres_page_sql(relation, &columns);
        let rows = self.client.query(&sql, &[&i64::from(limit), &i64::from(offset)])?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let mut cells = Vec::with_capacity(columns.len());
            for index in 0..columns.len() {
                let value: Option<String> = row.get(index);
                cells.push(value.unwrap_or_default());
            }
            out.push(cells);
        }
        Ok(Page {
            columns,
            rows: out,
            offset,
            limit,
        })
    }

    fn catalog_hint(&mut self) -> Result<Option<String>> {
        let profile = if self.has_table("catalog_meta")? {
            let rows = self
                .client
                .query("SELECT value FROM catalog_meta WHERE key = 'profile' LIMIT 1", &[])?;
            rows.first().and_then(|row| row.get::<_, Option<String>>(0))
        } else {
            None
        };
        let version = if self.has_table("schema_migrations")? {
            let rows = self
                .client
                .query("SELECT MAX(version)::text FROM schema_migrations", &[])?;
            rows.first()
                .and_then(|row| row.get::<_, Option<String>>(0))
                .and_then(|text| text.parse().ok())
        } else {
            None
        };
        Ok(format_hint(profile.as_deref(), version))
    }
}

/// True when the URL asks for TLS. This build cannot negotiate it.
pub fn url_requires_tls(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.contains("sslmode=require") || lower.contains("sslmode=verify-ca") || lower.contains("sslmode=verify-full")
}

/// Fill an empty password from `PGPASSWORD`. Does not replace a password already in the URL.
pub fn apply_password(config: &mut Config, from_env: Option<&str>) {
    if config.get_password().is_some() {
        return;
    }
    if let Some(password) = from_env.filter(|password| !password.is_empty()) {
        config.password(password);
    }
}

/// `user@host:port/db` with no password.
pub fn display_label(config: &Config) -> String {
    let host = match config.get_hosts().first() {
        Some(Host::Tcp(host)) => host.clone(),
        Some(Host::Unix(path)) => path.display().to_string(),
        None => "localhost".to_string(),
    };
    let port = config.get_ports().first().copied().unwrap_or(5432);
    let db = config.get_dbname().unwrap_or("");
    match config.get_user() {
        Some(user) if !user.is_empty() => format!("{user}@{host}:{port}/{db}"),
        _ => format!("{host}:{port}/{db}"),
    }
}

#[cfg(test)]
mod tests {
    use postgres::Config;

    use super::{
        LIST_COLUMNS_SQL,
        LIST_RELATIONS_SQL,
        apply_password,
        display_label,
        url_requires_tls,
    };

    #[test]
    fn list_sql_skips_system_schemas() {
        assert!(LIST_RELATIONS_SQL.contains("information_schema.tables"));
        assert!(LIST_RELATIONS_SQL.contains("pg_catalog"));
        assert!(LIST_COLUMNS_SQL.contains("ordinal_position"));
        assert!(!LIST_RELATIONS_SQL.to_ascii_lowercase().contains("insert "));
    }

    #[test]
    fn require_and_verify_modes_are_refused() {
        assert!(url_requires_tls("postgres://localhost/db?sslmode=require"));
        assert!(url_requires_tls("host=localhost sslmode=verify-full"));
        assert!(!url_requires_tls("postgres://localhost/db"));
        assert!(!url_requires_tls("postgres://localhost/db?sslmode=disable"));
    }

    #[test]
    fn label_drops_the_password_and_env_fills_a_missing_one() {
        let config: Config = "postgres://alice:secret@localhost:5432/study".parse().unwrap();
        let label = display_label(&config);
        assert_eq!(label, "alice@localhost:5432/study");
        assert!(!label.contains("secret"));
        assert!(config.get_password().is_some());

        let mut bare: Config = "postgres://alice@localhost:5432/study".parse().unwrap();
        assert!(bare.get_password().is_none());
        apply_password(&mut bare, Some("from-env"));
        assert_eq!(bare.get_password(), Some(b"from-env".as_slice()));

        let mut with_password = config;
        apply_password(&mut with_password, Some("other"));
        assert_eq!(with_password.get_password(), Some(b"secret".as_slice()));
    }

    #[test]
    fn live_connect_when_url_is_set() {
        let Ok(url) = std::env::var("DBSYM_PG_URL") else {
            return;
        };
        let mut source = super::PostgresSource::connect(&url).unwrap();
        let _ = crate::source::CatalogSource::relations(&mut source).unwrap();
    }
}
