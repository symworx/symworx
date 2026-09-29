// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Read-only terminal browser for a dbSym catalog.
//!
//! Opens a SQLite file or a Postgres URL and pages tables. It does not init,
//! apply, or ingest, and it does not open the LoadSym catalog. Postgres DDL
//! for dbSym is still not shipped; a Postgres URL is a database the operator
//! already has. Connections are read-only. TLS is not built in.
//!
//! Binary: `symdb-view`.

#![doc(html_root_url = "https://docs.rs/symworx-dbsym-tui")]

mod app;
mod cli;
mod error;
mod model;
#[cfg(feature = "postgres")]
mod postgres;
mod quote;
mod source;
#[cfg(feature = "sqlite")]
mod sqlite;
mod ui;

pub use cli::{
    Cli,
    Target,
    parse_args,
    usage,
};
pub use error::{
    Error,
    Result,
};
pub use model::{
    Column,
    PAGE_SIZE,
    Page,
    Relation,
};
pub use source::CatalogSource;

/// Open `target` and run the browser until quit.
pub fn browse(target: &Target) -> Result<()> {
    let mut source = open(target)?;
    ui::run(&mut *source)
}

fn open(target: &Target) -> Result<Box<dyn CatalogSource>> {
    match target {
        Target::Sqlite(path) => open_sqlite(path),
        Target::Postgres(url) => open_postgres(url),
    }
}

#[cfg(feature = "sqlite")]
fn open_sqlite(path: &std::path::Path) -> Result<Box<dyn CatalogSource>> {
    Ok(Box::new(sqlite::SqliteSource::open(path)?))
}

#[cfg(not(feature = "sqlite"))]
fn open_sqlite(path: &std::path::Path) -> Result<Box<dyn CatalogSource>> {
    let _ = path;
    Err(Error::InvalidParameter(
        "symdb-view was built without the sqlite feature".to_string(),
    ))
}

#[cfg(feature = "postgres")]
fn open_postgres(url: &str) -> Result<Box<dyn CatalogSource>> {
    Ok(Box::new(postgres::PostgresSource::connect(url)?))
}

#[cfg(not(feature = "postgres"))]
fn open_postgres(url: &str) -> Result<Box<dyn CatalogSource>> {
    let _ = url;
    Err(Error::InvalidParameter(
        "symdb-view was built without the postgres feature".to_string(),
    ))
}
