# symworx-dbsym-tui

Read-only terminal browser for a dbSym catalog. Binary: `symdb-view`.

Two panes: tables on the left, one page of rows on the right. SQLite opens a file. Postgres opens a URL the operator already has. This crate does not ship Postgres DDL, and it does not init, apply, or ingest.

This is not [`symview`](../symworx-tui/README.md) and not the LoadSym catalog. LoadSym stays a workflow inside `symview`. Schemas stay separate.

## Status

| Piece | State |
|-------|--------|
| SQLite browse | **Supported** (read-only file) |
| Postgres browse | **Supported** (`NoTls`; password from the URL or `PGPASSWORD`) |
| TLS (`sslmode=require`) | **Not started** |
| Writes, free-form SQL, export | **Not started** |
| dbSym Postgres DDL | **Not started** (still SQLite-only in [`symworx-dbsym`](../symworx-dbsym/README.md)) |

## Usage

```bash
cargo run -p symworx-dbsym-tui --bin symdb-view
cargo run -p symworx-dbsym-tui --bin symdb-view -- path/to/dbsym.sqlite
cargo run -p symworx-dbsym-tui --bin symdb-view -- --url postgres://user@localhost:5432/dbname
```

With no arguments the binary opens `./.dbsym/dbsym.sqlite`.

| Key | Action |
|-----|--------|
| `j` `k` or arrows | Move in the focused pane |
| `Enter` | Open the selected table |
| `h` `l` | Move the column cursor |
| `PgUp` `PgDn` | Page rows (100 at a time) |
| `/` | Filter table names |
| `r` | Reload the table list |
| `Alt-?` | Help |
| `Esc` | Clear filter, or back to the table list; at that list, Esc again quits |
| `Ctrl-Q` | Quit immediately |

`q` is not quit.

The SQLite file is opened with `SQLITE_OPEN_READ_ONLY` and `PRAGMA query_only = ON`. Postgres runs `SET default_transaction_read_only = on`. Queries are the browser's own `SELECT`s, with identifiers quoted.

## Features

| Feature | Default | Purpose |
|---------|---------|---------|
| `sqlite` | yes | `rusqlite` 0.32, bundled, same pin as dbSym |
| `postgres` | yes | Sync `postgres` client, `NoTls` |

## Citation

If you use this software, please cite it:

Berry, N. T. (2026). *SymWorx* [Computer software]. https://github.com/symworx/symworx

```bibtex
@software{Berry_SymWorx_2026,
  author  = {Berry, Nathaniel T.},
  license = {Apache-2.0},
  title   = {{SymWorx}},
  url     = {https://github.com/symworx/symworx},
  year    = {2026}
}
```

Add the version you used. [`CITATION.cff`](https://github.com/symworx/symworx/blob/main/CITATION.cff) and GitHub **Cite this repository** carry the current release.

## See also

- Catalog crate: [../symworx-dbsym/README.md](../symworx-dbsym/README.md)
- Design notes: [../symworx-dbsym/notes/design.md](../symworx-dbsym/notes/design.md)
