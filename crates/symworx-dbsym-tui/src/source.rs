// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Read-only catalog access. Drivers only run the statements they build.

use crate::{
    error::Result,
    model::{
        Column,
        Page,
        Relation,
    },
};

/// One open database. Methods take `&mut self` because the Postgres client does.
pub trait CatalogSource {
    /// Path, or `user@host:port/db` with the password removed.
    fn label(&self) -> &str;

    /// User tables, skipping driver catalogs (`sqlite_%`, `pg_catalog`, `information_schema`).
    fn relations(&mut self) -> Result<Vec<Relation>>;

    /// Columns of `relation` in table order.
    fn columns(&mut self, relation: &Relation) -> Result<Vec<Column>>;

    /// One page of display cells.
    fn page(&mut self, relation: &Relation, limit: u32, offset: u32) -> Result<Page>;

    /// `profile=… · schema v…` when those dbSym tables exist. `Ok(None)` for any other database.
    fn catalog_hint(&mut self) -> Result<Option<String>>;
}
