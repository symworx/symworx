# LoadSym Personal / velofit Starter

Blueprint for the personal ride archive at **`~/velofit`**, synced to **`s3:bitterbeta-useast1-velofit`**, with SymWorx tools for FIT load / NP-TSS and (later) a periodization DB.

**Key decisions:**
- Files: `.fit` primary (SRM PC8 email, Garmin, Polar). Historical `.pwx` kept in `raw/` only (no parser yet).
- Local root: `~/velofit` (override with `VELOFIT_HOME`)
- Cloud: `s3:bitterbeta-useast1-velofit` via `syncd velofit` (rclone bisync, same pattern as elibrary)
- Phase 1 (current): **archive + TUI** — no activity DB inserts yet
- Phase 2 (later): Postgres/SQLite catalog using `symworx-loadsym-db` schema

## Schema crate (`symworx-loadsym-db`)

- Zero runtime dependencies (`include_str!` of SQL + `SCHEMA_VERSION` + `get_schema`).
- `symload db print-schema` (feature `db` on `symworx-loadsym`).
- DB drivers / migrations stay in a separate personal app when you need them — not in this crate.

```rust
use symworx_loadsym_db;
let sql = symworx_loadsym_db::get_schema("postgres");
```

## Layout

```text
~/velofit/
  raw/       # primary archive (mirrors S3 raw/ + promoted imports)
  inbox/     # email / manual drop; promote into raw/
  .tmp/      # excluded from bisync
```

Historical bucket content is flat under `raw/` (~2.5k `.fit`, ~900 `.pwx`). Do not reorganize into `YYYY/MM` until a later migration phase (avoids bisync rename storms).

## Sync (`syncd velofit`)

Targets live in `~/worx/ntberry/sysmgmt/bash/bashrc.d/bin/syncd.sh`.

```bash
# First time (after local tree exists): prefer copy then resync
toolbox run --container cloud-sync rclone copy \
  s3:bitterbeta-useast1-velofit/raw ~/velofit/raw --progress

# Establish bisync state once both sides known
syncd resync velofit

# Ongoing
syncd velofit
```

Bisync script: `~/.local/bin/velofit-bisync.sh`  
Unit: `~/.config/systemd/user/velofit-bisync.service`  
Excludes: `.tmp/**`, `*.eml`

## Email → inbox (SRM PC8)

```bash
export SYMLOAD_USER="nberry.fitdata@gmail.com"
export SYMLOAD_APP_PASSWORD="your-app-password"

cargo run -p symworx-loadsym --features "fit,email,db" -- email fetch
# default target: ~/velofit/inbox

cargo run -p symworx-loadsym --features fit -- inbox promote
# moves unique .fit inbox → raw

syncd velofit
```

MIME attachments are decoded (not raw RFC822). Re-fetch skips existing basenames.

## Stats / TUI

```bash
# Headless
cargo run -p symworx-loadsym --features fit -- stats ~/velofit/raw --ftp 280

# Interactive
cargo run -p symworx-tui --bin symview
# Home → 2 LoadSym → Workout → i/a  loads newest .fit from ~/velofit + ./data
```

Search roots (via `symworx_io::default_activity_search_dirs`):
1. `~/velofit/inbox`
2. `~/velofit/raw`
3. `./data`, `./rides`, `./training`

## Schema tables (for phase 2)

See `sql/schema.sql` — `activities`, `daily_loads`, `load_metrics` (ACWR + CTL/ATL/TSB), `power_bests`, `zones`, `ftp_history`, `athlete`, `daily_context`, `planned_workouts`.

Phase 1 does **not** write to a DB; use `symload stats` and LoadSym for metrics.

## Recommended daily loop

1. `symload email fetch` (when new SRM mail arrives)
2. `symload inbox promote`
3. `syncd velofit`
4. Optional: open `symview` LoadSym Workout (`i`) on newest ride

## Out of scope (this phase)

- Postgres/SQLite activity catalog
- PWX reader
- Nested date archive reorganization
- Full TUI multi-file browser
