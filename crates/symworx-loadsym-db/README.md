# symworx-loadsym-db

**Schema-only** crate for a personal training-load catalog used by LoadSym / `symload`.

- **Zero runtime dependencies** — embeds SQL via `include_str!`
- **No personal data** — never sample athlete rows, emails, or real paths beyond generic `$VELOFIT_HOME`
- **Dialects:** SQLite (recommended personal default) and PostgreSQL

The actual database file lives **outside** this repository (typically `$VELOFIT_HOME/db/loadsym.sqlite`).

## Schema version

Current: **`SCHEMA_VERSION = 4`**

| Version | What landed |
|---------|-------------|
| v1 | Initial catalog (`activities`, `daily_loads`, `load_metrics`, …) |
| v2 | Time-varying `ftp_history` + `activities.ftp_history_id` / `ftp_used_w` |
| v3 | `catalog_meta` key/value (e.g. `last_ingest_at` ingest watermark) |
| v4 | Multi-source session linking: `ingest_pipeline`, `external_id`, `session_group_id`, `counts_for_load`, `is_primary`, `match_reason`, `session_groups` |

Migrations for existing SQLite files are applied by `symload` (`catalog::migrate_catalog`) on `db init` / `open_catalog`. Documented SQL sketches live under `sql/migrations/`.

## Files

| Path | Role |
|------|------|
| `sql/schema.sqlite.sql` | Canonical SQLite DDL (personal) |
| `sql/schema.sql` | PostgreSQL DDL (shared / multi-user) |
| `sql/migrations/` | Documented upgrade sketches (`002_…`, `004_…`) |
| `docs/loadsym-personal-starter.md` | Operator guide: archive layout, ingest, email, Polar, sync, privacy |
| `src/lib.rs` | `get_schema`, `SCHEMA_VERSION`, `DEFAULT_DB_RELATIVE` |

## Usage

```rust
use symworx_loadsym_db::{get_schema, SCHEMA_VERSION, DEFAULT_DB_RELATIVE};

let sql = get_schema("sqlite"); // or "postgres"
assert!(SCHEMA_VERSION >= 4);
// Personal file: $VELOFIT_HOME/db/loadsym.sqlite  (DEFAULT_DB_RELATIVE)
```

```bash
# One-shot dirs + empty catalog (from workspace root)
./scripts/init-velofit.sh

# Print SQL only (no driver)
cargo run -p symworx-loadsym --features db -- db print-schema --sqlite

# Create / migrate personal catalog (needs rusqlite)
cargo run -p symworx-loadsym --features sqlite -- db init
cargo run -p symworx-loadsym --features sqlite -- db status
```

Runtime init, ingest, session relink, and multi-pipeline sync live in **`symworx-loadsym`** (`catalog` + `symload` binary), not in this crate.

## Multi-source load counting (v4)

The catalog may store **multiple FIT copies** of the same real-world session (e.g. SRM via email + Polar AccessLink). Only rows with **`counts_for_load = 1`** roll into `daily_loads` (and thus ACWR / PMC).

| Column / table | Meaning |
|----------------|---------|
| `ingest_pipeline` | How the file was obtained: `email` \| `polar` \| `manual` \| … |
| `source_platform` | Device family from FIT: `srm`, `polar`, `garmin`, … |
| `external_id` | Provider id (e.g. Polar exercise hash from `polar_{id}.fit`) |
| `session_group_id` | Links duplicate copies of one session |
| `counts_for_load` | `1` = counts toward daily TSS / ride_count; `0` = archive only |
| `is_primary` | Load primary within the group |
| `session_groups` | Group metadata (`primary_activity_id`, `match_method`) |

Matching and primary selection (power-meter preferred) are implemented in `symworx-loadsym::catalog` (`relink` / end of `ingest`). See the personal starter for CLI and TUI calendar badges (`●` / `○`).

## Privacy

| In this crate (public) | Outside the repo (yours) |
|------------------------|---------------------------|
| SQL strings, version constants | `*.sqlite`, FIT archives |
| Generic `$VELOFIT_HOME` docs | `.env`, `polar_token.json`, IMAP passwords |
| Migration sketches | rclone remotes, systemd with real paths |

Never commit personal catalog files or secrets into SymWorx.

## See also

- **Operator guide:** [docs/loadsym-personal-starter.md](docs/loadsym-personal-starter.md)
- **Algorithms + CLI:** [../symworx-loadsym/README.md](../symworx-loadsym/README.md)
- **Workspace overview:** [../../README.md](../../README.md)
