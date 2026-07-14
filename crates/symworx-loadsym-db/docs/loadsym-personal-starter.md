# LoadSym Personal Archive + Catalog Starter

Blueprint for a **personal** ride archive and SQLite catalog used with SymWorx.
All personal data stays on your machine (or your private cloud). **Nothing in this document is a real account, email, or bucket name.**

**Key decisions:**
- Files: `.fit` primary (SRM PC8 email, Garmin, Polar). Historical `.pwx` may sit in `raw/` unparsed.
- Local root: `$VELOFIT_HOME` (default `~/velofit`)
- Catalog: SQLite at `$VELOFIT_HOME/db/loadsym.sqlite` (override with `SYMLOAD_DB`)
- Cloud sync: your own rclone / bisync setup (private); do not commit remotes or credentials to SymWorx
- Open-source SymWorx ships **schema + tools only** — never sample athlete rows or personal paths beyond generic `~/velofit`

## Privacy boundary

| Outside the repo (yours) | Inside SymWorx (public) |
|--------------------------|-------------------------|
| `$VELOFIT_HOME/raw`, `inbox`, `db/*.sqlite` | SQL schema strings |
| IMAP username / app password | Env var *names* only |
| S3 / rclone config, systemd units | Generic CLI (`symload`) |
| Athlete profile, notes, real FTP history | NP/TSS/ACWR algorithms |

Never commit `.env`, `*.sqlite`, or FIT archives into the SymWorx tree.

## Layout

```text
$VELOFIT_HOME/          # default: ~/velofit
  raw/                  # FIT archive
  inbox/                # email / manual drop
  db/
    loadsym.sqlite      # personal catalog (not in git)
  .tmp/                 # scratch (exclude from sync if desired)
  .env                  # optional secrets (chmod 600; never commit)
```

## Schema crate (`symworx-loadsym-db`)

Zero runtime dependencies. Embeds Postgres + SQLite SQL via `include_str!`.

```rust
use symworx_loadsym_db::{get_schema, SCHEMA_VERSION, DEFAULT_DB_RELATIVE};
let sql = get_schema("sqlite"); // or "postgres"
```

```bash
cargo run -p symworx-loadsym --features db -- db print-schema --sqlite
cargo run -p symworx-loadsym --features sqlite -- db init
# → creates $VELOFIT_HOME/db/loadsym.sqlite
```

## Ingest

```bash
# After FITs are in raw/
export VELOFIT_HOME="$HOME/velofit"   # optional if using default
export SYMLOAD_DB="$HOME/velofit/db/loadsym.sqlite"  # optional

cargo run -p symworx-loadsym --features sqlite -- ingest --ftp 280
# or: ingest /path/to/one.fit --db /path/to/loadsym.sqlite

cargo run -p symworx-loadsym --features sqlite -- db status
```

Ingest stores **relative** keys when files live under `$VELOFIT_HOME` (e.g. `raw/ride.fit`), not absolute home paths. Dedup uses `file_hash` (SHA-256) and `source_file` UNIQUE.

## Email → inbox (optional)

IMAP + MIME extraction lives in **`symworx-io`** (`email` feature). The `symload` CLI only orchestrates credentials → fetch → `$VELOFIT_HOME/inbox`.

```bash
export SYMLOAD_USER="you@example.com"          # placeholder
export SYMLOAD_APP_PASSWORD="your-app-password" # never commit

cargo run -p symworx-loadsym --features "fit,email" -- email fetch
# Optional IMAP SEARCH (default: SUBJECT SRM for PC8 exports)
cargo run -p symworx-loadsym --features "fit,email" -- email fetch --query "OR SUBJECT SRM SUBJECT Polar"
cargo run -p symworx-loadsym --features fit -- inbox promote
cargo run -p symworx-loadsym --features sqlite -- ingest --ftp 280
# ingest also fills load_metrics.ctl / atl / tsb via the PMC pulse-response model
```

## Stats / TUI

```bash
cargo run -p symworx-loadsym --features fit -- stats "$VELOFIT_HOME/raw" --ftp 280
cargo run -p symworx-tui --bin symview
# Home → LoadSym → Workout → i/a  loads newest .fit under $VELOFIT_HOME + ./data
# Home → LoadSym → Calendar (2)  reads daily_loads + load_metrics from db/loadsym.sqlite
#   r reloads catalog   g synthetic demo
```

## Sync (private machine only)

Wire rclone bisync for `$VELOFIT_HOME` in your own sysmgmt scripts. Optionally exclude `db/**` if you prefer re-ingest over syncing the SQLite file. Exclude `.tmp/**` and `*.eml`.

## Schema tables

`activities` (includes `ftp_used_w`, `ftp_history_id`), `daily_loads`, `load_metrics`, `power_bests`, `zones`, **`ftp_history`** (time-varying FTP for scoring/re-scoring), `athlete`, `daily_context`, `planned_workouts`, view `recent_load`.

### FTP history

```bash
# Record FTP changes (example placeholders)
symload ftp set --date 2018-01-01 --ftp 260 --source estimate
symload ftp set --date 2022-06-15 --ftp 280 --source 20min_test
symload ftp list

# Re-score all rides using history (fallback FTP if no row covers that date)
symload reprocess --ftp 280
```

Lookup rule: for ride date `D` and sport `S`, use the latest `ftp_history` row with
`effective_from <= D` and (`effective_to` is null or `> D`). Store the applied value
on the activity as `ftp_used_w` / `ftp_history_id`.

See `sql/schema.sqlite.sql` and `sql/schema.sql` (schema version 2).
