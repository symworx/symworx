# LoadSym Personal / Periodization Starter (Separate Project)

This document provides the blueprint + copy-pasteable starters for the **separate repo** that will own your ride archive, Postgres DB, ingestion, rclone sync to AWS, and periodization logic. It calls into SymWorx (via Rust crates or Python bindings).

**Key decisions (from your answers)**:
- Files: .fit (SRM PC8 email exports, Garmin, Polar)
- Project style: Hybrid / TBD (Python orchestration + Rust via bindings or CLI)
- DB: PostgreSQL
- Add NP/TSS/IF: done (in symworx-loadsym)

## Feedback on "symworx crate for DB initialization"

The schema helpers are provided via the `symworx-loadsym-db` crate (or the `db` feature of `symworx-loadsym`).

- Zero runtime dependencies (just `include_str!` of the SQL + a getter + `SCHEMA_VERSION`).
- This sidesteps the main caveats (no heavy sqlx/diesel/tokio, no forcing a DB driver choice).
- The separate project can depend on it (path or crates.io later) to get the exact SQL.
- `symload db print-schema [--dialect postgres|sqlite]` is now provided by the `symworx-loadsym` crate (via the `db` feature).

**Why not a heavier "init" crate?**
- Avoids pulling DB libs into the symworx ecosystem (per AGENTS.md hygiene).
- Schema evolution is owned by *your* separate app's migrations.
- SQLite vs Postgres: the crate gives you the SQL for either; applying it is one-liner in your project.

If later we want a small optional feature for applying the schema (behind a `postgres` feature using a thin client), we can add it.

## Recommended Layout (your new repo, e.g. ~/worx/loadsym-personal or athlete-load)

```
loadsym-personal/
  README.md
  pyproject.toml
  src/
    loadsym_personal/
      __init__.py
      cli.py
      ingest.py
      db.py
      metrics.py
      periodize.py
  sql/
    schema.sql
  scripts/
    sync-training.sh
    watch-inbox.sh
  data/                 # local only (gitignored)
    inbox/
    archive/rides/      # YYYY/MM/*.fit
  .gitignore
```

## 1. rclone Sync Script (modeled on your existing bisyncs)

`scripts/sync-training.sh` (adapt paths/remote):

```bash
#!/usr/bin/env bash
set -euo pipefail

# Local training archive (fits + derived)
LOCAL="$HOME/symload/archive"
REMOTE="s3:bitterbeta-training-archive"   # or your bucket/path
LOG_DIR="$HOME/.local/share/rclone"
mkdir -p "$LOG_DIR"

toolbox run --container cloud-sync \
  rclone bisync "$LOCAL" "$REMOTE" \
    --verbose \
    --log-file="$LOG_DIR/training-bisync.log" \
    --exclude ".tmp/**" \
    --create-empty-src-dirs
```

Usage like your others: `syncd` wrapper or direct. Keep "inbox" local-only; only archive/ is synced.

## 2. Schema (power + load focused)

See `sql/schema.sql` in the symworx-loadsym-db crate (or run `cargo run -p symworx-loadsym --features db -- db print-schema`).

The schema is also available programmatically via the lightweight crate `symworx-loadsym-db` (no heavy DB drivers):

```rust
use symworx_loadsym_db;
let sql = symworx_loadsym_db::get_schema("postgres");
```

Key tables added/enhanced after looking at TrainingPeaks (PMC/CTL/ATL/TSB + planned workouts), GoldenCheetah (power_bests / MMP curves), etc.:
- `activities` (rich ride data including NP/TSS + device + workout_type + tags)
- `power_bests` (best power for standard durations — enables power curve tracking)
- `load_metrics` (both ACWR and CTL/ATL/TSB)
- `zones`, `planned_workouts`, `athlete`, `daily_context` (wellness + RPE)
- `ftp_history`, `daily_loads`

See the .sql file header for rationale.

```sql
CREATE TABLE IF NOT EXISTS activities (
    id BIGSERIAL PRIMARY KEY,
    source_file TEXT UNIQUE NOT NULL,
    ride_date DATE NOT NULL,
    duration_s DOUBLE PRECISION,
    tss DOUBLE PRECISION,
    np DOUBLE PRECISION,
    intensity_factor DOUBLE PRECISION,
    avg_power DOUBLE PRECISION,
    max_power DOUBLE PRECISION,
    avg_hr DOUBLE PRECISION,
    sport TEXT,
    manufacturer TEXT,
    device_product TEXT,
    created_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE IF NOT EXISTS daily_loads (
    ride_date DATE PRIMARY KEY,
    load_value DOUBLE PRECISION NOT NULL,  -- usually TSS sum or chosen proxy
    source TEXT,
    updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE IF NOT EXISTS player_load_metrics (
    ride_date DATE PRIMARY KEY,
    acute_load DOUBLE PRECISION,
    chronic_load DOUBLE PRECISION,
    acwr DOUBLE PRECISION,
    risk_level TEXT,
    monotony DOUBLE PRECISION,
    strain DOUBLE PRECISION
);

-- indexes etc.
CREATE INDEX IF NOT EXISTS idx_activities_date ON activities(ride_date);
```

Init: `psql -f sql/schema.sql`

## 3. Python Ingestion Skeleton (uses symworx when available)

Install bindings first (from symworx-tui workspace):

```bash
cd /path/to/symworx-tui/bindings/python
maturin develop --manifest-path ../../crates/symworx-io/Cargo.toml ... # or the workspace pyproject
# similarly for loadsym
```

`src/loadsym_personal/ingest.py` (starter):

```python
from pathlib import Path
import os
import psycopg2
from datetime import date
# Prefer symworx when built; fallback to other parsers
try:
    from symworx.core.io import load_activity as sym_load_activity
    from symworx.load import compute_ride_metrics  # if exposed at top
    HAS_SYMWORX = True
except Exception:
    HAS_SYMWORX = False
    # import fitparse or similar as fallback

def parse_fit(path: Path):
    if HAS_SYMWORX:
        d = sym_load_activity(str(path))
        # d is dict-like with power_w, times etc.
        # compute metrics client side or server
        return d
    # TODO: fallback
    return {}

def compute_tss_from_dict(act_dict, ftp=300.0):
    # re-implement minimal or call Rust via pyo3 if bound
    # For now return placeholder using avg* time heuristic
    return 50.0

def write_activity(conn, path: Path, act: dict, tss: float):
    with conn.cursor() as cur:
        cur.execute("""
            INSERT INTO activities (source_file, ride_date, duration_s, tss, ...)
            VALUES (%s, %s, %s, %s, ...)
            ON CONFLICT (source_file) DO UPDATE SET tss=EXCLUDED.tss
        """, (str(path), date.today(), act.get("duration_s"), tss))
    conn.commit()

def process_inbox(inbox: Path, db_url: str, ftp: float = 300.0):
    conn = psycopg2.connect(db_url)
    for p in sorted(inbox.glob("*.fit")):
        act = parse_fit(p)
        tss = compute_tss_from_dict(act, ftp)
        write_activity(conn, p, act, tss)
        # move or hardlink into archive/YYYY/MM/
        print("ingested", p)
    conn.close()
```

CLI (typer or argparse) + `watch` mode (watchdog or simple poll).

Call periodization:

```python
from loadsym.modeling.programming.daily import daily_programming
# feed readiness + context from your DB + acute/chronic
```

## 4. Deriving Daily Loads for ACWR

After ingest:
- Sum TSS (or chosen load) per day into `daily_loads`.
- Recompute ACWR / monotony using the functions from symworx (or port the Rust math).
- Store snapshots in `player_load_metrics`.

TUI can either:
- Read a `daily_loads.csv` export from this project, or
- Point at `archive/` and recompute on the fly (already partially supported).

## 5. Email → Inbox (SRM PC8)

Implementation started (use `--features email`).

**Recommended layout**:
- Your separate DB/periodization project at `~/symload`
- Downloaded .fit files go to `~/symload/inbox`

**Setup**:
1. Enable IMAP in Gmail + generate an App Password.
2. `export SYMLOAD_USER="user@example.com"`
   `export SYMLOAD_APP_PASSWORD="your-app-password"`
3. `cargo build -p symworx-loadsym --features "email,db"`
4. `symload email fetch ~/symload/inbox`

It searches messages with "SRM" in the subject and extracts .fit attachments.

See the examples in `crates/symworx-loadsym/README.md` and this file.

## Next Steps After Scaffolding Your Repo

1. `cargo check` / build the symworx crates in the main workspace.
2. Build the Python symworx wheels/bindings you need.
3. `psql` init schema.
4. Drop a real .fit in inbox, run ingest script.
5. Query DB + verify TSS/ACWR written.
6. Use `symview` (2 → LoadSym → i) on the same or copied .fit for immediate viz + NP/TSS.
7. Iterate the separate project with more periodization (use loadsym Python modeling code for daily/weekly programming + readiness).

See also the main AGENTS.md and `crates/symworx-loadsym/README.md` + TUI LoadSym help.
