# symworx-dbsym

Per-study (or edge) **research catalog**: subjects, sessions, flexible attributes,
ingest audit, and file provenance.

This crate is the registry. Analysis stays in the other crates. Signal bytes
still load and save through [`symworx-io`](../symworx-io/README.md).

`symdb init <project>` copies a **template** schema to
`<project>/.dbsym/schema.sqlite.sql`. Edit that file in the study repo (commit
it). The engine applies the study copy, not the crate, after install. Init
never overwrites an existing study schema. Re-apply with `symdb apply`.

The SQLite file (`dbsym.sqlite`) and `objects/` stay gitignored. Never commit
study data, hashes, or credentials.

The crate template matches the [`symworx-loadsym-db`](../symworx-loadsym-db/README.md)
shape (`include_str!`, `SCHEMA_VERSION`). Domain errors use `thiserror` like
LoadSym / embed. `symworx-error::SymError` is the workspace I/O type; this crate
does not depend on it by default (it pulls `csv`).

## Why

Biosignal and lab work already has algorithms here. What it lacks is a small
store so a study (or an edge node) can answer: *which subject, which session,
which file, what attributes, and what was ingested.*

The same DDL covers a multi-subject study directory and a one-subject edge node.

This is **not** the LoadSym personal ride catalog.

## Status

| Piece | State |
|-------|--------|
| Schema v1 (SQLite) | **Supported** |
| `init` / seed / `symdb` CLI | **Supported** (`--features sqlite`) |
| File register / CSV ingest / export | **Not started** |
| TUI workflow | **Planned** |
| Postgres dialect | **Not started** (LoadSym already has the dual-SQL pattern) |

Design: [notes/design.md](notes/design.md).

## Usage

```rust
use symworx_dbsym::{get_schema, SCHEMA_VERSION, DEFAULT_DB_RELATIVE, StoreProfile};

let sql = get_schema("sqlite");
assert!(SCHEMA_VERSION >= 1);
assert_eq!(DEFAULT_DB_RELATIVE, ".dbsym/dbsym.sqlite");
assert_eq!(StoreProfile::parse("study"), Some(StoreProfile::Study));
```

```bash
cargo test -p symworx-dbsym
cargo test -p symworx-dbsym --features sqlite

# Empty store in a project directory (data stays in that dir)
cargo run -p symworx-dbsym --features sqlite --bin symdb -- init ./scratch --profile study --preset exercise_science
# edit ./scratch/.dbsym/schema.sqlite.sql  then:
cargo run -p symworx-dbsym --features sqlite --bin symdb -- apply ./scratch
cargo run -p symworx-dbsym --features sqlite --bin symdb -- status ./scratch
```

## Schema version

Current: **`SCHEMA_VERSION = 1`** — `subjects`, `sessions`, `ingest_batch`,
`attributes`, `observations` / `observation_values`, `file_records`,
`catalog_meta`.

EAV is for attributes and assay points (`time_s` on `observations`). Time series
stay as files; the catalog holds provenance.

## Features

| Feature | Default | Purpose |
|---------|---------|---------|
| `sqlite` | no | `rusqlite` (bundled, same pin as LoadSym) + init / seed / CLI |

## See also

- Design notes: [notes/design.md](notes/design.md)
- LoadSym schema (different catalog): [../symworx-loadsym-db/README.md](../symworx-loadsym-db/README.md)
- Workspace overview: [../../README.md](../../README.md)
