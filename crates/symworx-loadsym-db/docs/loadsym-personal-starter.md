# LoadSym Personal Archive + Catalog Starter

Blueprint for a **personal** ride archive and SQLite catalog used with SymWorx.
All personal data stays on your machine (or your private cloud). **Nothing in this document is a real account, email, or bucket name.**

**Key decisions:**

- Files: `.fit` primary (SRM PC8 email, Garmin, Polar AccessLink, manual). Historical `.pwx` may sit in `raw/` unparsed.
- Local root: `$VELOFIT_HOME` (default `~/velofit`)
- Catalog: SQLite at `$VELOFIT_HOME/db/loadsym.sqlite` (override with `SYMLOAD_DB`)
- Cloud file sync: your own rclone / bisync setup (private); do not commit remotes or credentials to SymWorx
- Open-source SymWorx ships **schema + tools only** — never sample athlete rows or personal paths beyond generic `~/velofit`

**Related crates:**

| Crate | Role |
|-------|------|
| `symworx-loadsym-db` | Schema SQL only (`SCHEMA_VERSION`, migrations) — [crate README](../README.md) |
| `symworx-loadsym` | Algorithms, `catalog` runtime, `symload` CLI |
| `symworx-io` | FIT load, IMAP (`email`), AccessLink (`polar`) |
| `symworx-tui` | `symview` → Home → LoadSym (Calendar / Workout / Metrics) |

---

## Privacy boundary

| Outside the repo (yours) | Inside SymWorx (public) |
|--------------------------|-------------------------|
| `$VELOFIT_HOME/raw`, `inbox`, `db/*.sqlite` | SQL schema strings |
| IMAP username / app password | Env var *names* only |
| Polar client secret / `polar_token.json` | Env var *names* only |
| S3 / rclone config, systemd units | Generic CLI (`symload`) |
| Athlete profile, notes, real FTP history | NP/TSS/ACWR algorithms |

Never commit `.env`, `polar_token.json`, `*.sqlite`, or FIT archives into the SymWorx tree.

---

## Layout

```text
$VELOFIT_HOME/              # default: ~/velofit
  raw/                      # FIT archive (ingest scans recursively)
    email/                  # promoted IMAP attachments (default promote target)
    polar/                  # AccessLink downloads: polar_{exerciseId}.fit
    manual/                 # optional hand drops
  inbox/                    # email fetch staging
  db/
    loadsym.sqlite          # personal catalog (not in git)
  polar_token.json          # AccessLink token after `polar auth` (chmod 600)
  .tmp/                     # scratch (exclude from cloud sync if desired)
  .env                      # secrets (chmod 600; never commit)
```

Flat FITs under `raw/*.fit` still work; subdirs improve **pipeline provenance** (`ingest_pipeline`).

---

## Schema crate (`symworx-loadsym-db`)

Zero runtime dependencies. Embeds Postgres + SQLite SQL via `include_str!`.

**Current schema version: 4** (multi-source session linking).

```rust
use symworx_loadsym_db::{get_schema, SCHEMA_VERSION, DEFAULT_DB_RELATIVE};
let sql = get_schema("sqlite"); // or "postgres"
assert_eq!(SCHEMA_VERSION, 4);
// DEFAULT_DB_RELATIVE == "db/loadsym.sqlite"
```

```bash
cargo run -p symworx-loadsym --features db -- db print-schema --sqlite
cargo run -p symworx-loadsym --features sqlite -- db init
# → creates/migrates $VELOFIT_HOME/db/loadsym.sqlite
```

| Version | Contents |
|---------|----------|
| v1 | Base tables (`activities`, `daily_loads`, `load_metrics`, …) |
| v2 | `ftp_history` + activity FTP linkage |
| v3 | `catalog_meta` (e.g. `last_ingest_at`) |
| v4 | `counts_for_load`, `session_groups`, pipeline / external ids |

SQL sources: `sql/schema.sqlite.sql`, `sql/schema.sql`, sketches in `sql/migrations/`.

---

## Ingest

```bash
export VELOFIT_HOME="$HOME/velofit"   # optional if using default
# export SYMLOAD_DB="$HOME/velofit/db/loadsym.sqlite"  # optional

cargo run -p symworx-loadsym --features sqlite -- ingest --ftp 280
# Incremental by default: only files with mtime >= catalog_meta.last_ingest_at
# Full recheck (still skips known file_hash unless --force):
cargo run -p symworx-loadsym --features sqlite -- ingest --all --ftp 280
# Re-score all candidates:
cargo run -p symworx-loadsym --features sqlite -- ingest --force --ftp 280
# or: ingest /path/to/one.fit --db /path/to/loadsym.sqlite

cargo run -p symworx-loadsym --features sqlite -- db status
# activities (load_primary vs secondary), multi-source groups, last_ingest_at
```

**Behavior:**

- Directory ingest is **recursive** (finds `raw/email/`, `raw/polar/`, …).
- Relative `source_file` keys when under `$VELOFIT_HOME` (e.g. `raw/polar/polar_ABC.fit`).
- Content dedup: `file_hash` (SHA-256) + `source_file` UNIQUE.
- End of ingest runs **session relink** then rebuilds `daily_loads` / `load_metrics` (PMC CTL/ATL/TSB).
- `daily_loads` sums only activities with **`counts_for_load = 1`**.

### Multi-source sessions (schema v4+)

The same real-world workout may arrive from more than one pipeline (e.g. SRM email FIT + Polar AccessLink FIT). Both can be stored; only one counts for load.

| Concept | Behavior |
|---------|----------|
| `ingest_pipeline` | How obtained: `email` / `polar` / `manual` |
| `source_platform` | Device family from FIT: `srm`, `polar`, `garmin`, … |
| `external_id` | Provider id (Polar: from `polar_{id}.fit` filename) |
| `counts_for_load` | `1` → `daily_loads` / ACWR / PMC; `0` → archive copy |
| Matching | Time-window (start ±10 min, similar duration, compatible sport) |
| Primary preference | Power-meter / email over Polar watch when both match |

```bash
# Rebuild groups on an existing catalog (no re-download):
cargo run -p symworx-loadsym --features sqlite -- relink
```

Calendar (TUI LoadSym → Calendar): **`●`** load primary, **`○`** secondary/dup, with a short pipeline/platform label.

---

## Email → inbox (optional)

IMAP + MIME live in **`symworx-io`** (`email` feature). CLI orchestrates credentials → fetch → `$VELOFIT_HOME/inbox`.

Host-side and reproducible: `$VELOFIT_HOME/.env` (gitignored). No AI/MCP dependency.

```bash
# $VELOFIT_HOME/.env (never commit)
# SYMLOAD_USER=...
# SYMLOAD_APP_PASSWORD=...
# SYMLOAD_IMAP_HOST=imap.gmail.com   # optional

cargo run -p symworx-loadsym --features "fit,email" -- email fetch
# Optional IMAP SEARCH (default: SUBJECT SRM for PC8 exports)
cargo run -p symworx-loadsym --features "fit,email" -- email fetch --query "OR SUBJECT SRM SUBJECT Polar"

# Default promote target: raw/email/ (pipeline provenance)
cargo run -p symworx-loadsym --features fit -- inbox promote
# Override: inbox promote --to $VELOFIT_HOME/raw

cargo run -p symworx-loadsym --features sqlite -- ingest --ftp 280
```

---

## Polar AccessLink → raw/polar/ (optional)

Official third-party API for Polar Flow training data: [AccessLink v3](https://www.polar.com/accesslink-api/).

1. Create a client at [admin.polaraccesslink.com](https://admin.polaraccesslink.com).
2. Register redirect URI **`http://127.0.0.1:8765/callback`** (or match `POLAR_REDIRECT_URI`).
3. Put credentials in `$VELOFIT_HOME/.env`:

```bash
# $VELOFIT_HOME/.env
POLAR_CLIENT_ID=...
POLAR_CLIENT_SECRET=...
# POLAR_REDIRECT_URI=http://127.0.0.1:8765/callback
# POLAR_MEMBER_ID=local-symload
```

```bash
# One-time OAuth → $VELOFIT_HOME/polar_token.json
cargo run -p symworx-loadsym --features polar -- polar auth
cargo run -p symworx-loadsym --features polar -- polar status

# Recent exercise FITs (~last 30 days per AccessLink list docs)
cargo run -p symworx-loadsym --features polar -- polar fetch
# → raw/polar/polar_{exerciseId}.fit

cargo run -p symworx-loadsym --features sqlite -- ingest --ftp 280
```

**Limits:**

- List/FIT is **recent history** only (docs cite ~30 days for non-transactional exercises). Older history: manual Flow export into `raw/` (or `raw/manual/`).
- `external_id` from filename enables idempotent re-fetch.
- Webhooks need a public URL — not required for personal poll-based sync.

---

## Unified sync (email + Polar + ingest)

```bash
cargo build -p symworx-loadsym --release --features "email,polar,sqlite"

# Full pipeline: email fetch → promote → polar fetch → ingest + relink
./target/release/symload sync --ftp 280

symload sync --sources email --ftp 280
symload sync --sources polar --ftp 280 --skip-ingest
symload sync --sources polar,ingest --ftp 280 --all
```

Steps are **feature-gated**: missing `email` / `polar` / `sqlite` at build time skips that step with a message.

### Optional systemd user timer (private machine)

```ini
# ~/.config/systemd/user/symload-sync.service
[Unit]
Description=Symload multi-source training ingest

[Service]
Type=oneshot
Environment=VELOFIT_HOME=%h/velofit
# Prefer a fixed release binary; secrets stay in $VELOFIT_HOME/.env
ExecStart=%h/bin/symload sync --ftp 280
```

```ini
# ~/.config/systemd/user/symload-sync.timer
[Unit]
Description=Run symload sync twice daily

[Timer]
OnCalendar=*-*-* 07:30:00
OnCalendar=*-*-* 19:30:00
Persistent=true

[Install]
WantedBy=timers.target
```

```bash
systemctl --user enable --now symload-sync.timer
systemctl --user list-timers | grep symload
```

Cron alternative:

```cron
30 7,19 * * * VELOFIT_HOME=$HOME/velofit $HOME/bin/symload sync --ftp 280 >>$HOME/velofit/.tmp/sync.log 2>&1
```

---

## Stats / TUI

```bash
cargo run -p symworx-loadsym --features fit -- stats "$VELOFIT_HOME/raw" --ftp 280
cargo run -p symworx-tui --bin symview
# Home → LoadSym → Workout → i/a  loads newest .fit under $VELOFIT_HOME + ./data
# Home → LoadSym → Calendar (2)  daily_loads + load_metrics from db/loadsym.sqlite
#   r reloads catalog   g synthetic demo
#   Day rides: ● primary  ○ secondary/dup  + pipeline label
```

---

## File cloud sync (private machine only)

Wire rclone bisync for `$VELOFIT_HOME` in your own scripts. Optionally exclude `db/**` if you prefer re-ingest over syncing SQLite. Exclude `.tmp/**`, `*.eml`, and never sync `.env` / `polar_token.json` to untrusted remotes.

---

## Schema tables (overview)

| Table / view | Role |
|--------------|------|
| `schema_migrations` | Applied schema version |
| `catalog_meta` | Pipeline state (`last_ingest_at`, …) |
| `activities` | One row per FIT; provenance + load flags (v4) |
| `session_groups` | Multi-source session groups (v4) |
| `daily_loads` | Per-day TSS / duration / ride_count (**load primaries only**) |
| `load_metrics` | ACWR, monotony/strain, PMC CTL/ATL/TSB |
| `power_bests` | Mean maximal power by duration |
| `zones` | Power/HR zone definitions |
| `ftp_history` | Time-varying FTP for scoring / reprocess |
| `athlete` | Optional profile |
| `daily_context` | Wellness (sleep, RPE, HRV, …) — not auto-filled by Polar yet |
| `planned_workouts` | Future plans |
| `recent_load` | Convenience view |

### FTP history

```bash
symload ftp set --date 2018-01-01 --ftp 260 --source estimate
symload ftp set --date 2022-06-15 --ftp 280 --source 20min_test
symload ftp list
symload reprocess --ftp 280   # re-score using history; --ftp is fallback
```

Lookup: for ride date `D` and sport `S`, latest `ftp_history` row with  
`effective_from <= D` and (`effective_to` is null or `> D`).  
Stored on the activity as `ftp_used_w` / `ftp_history_id`.

---

## Environment reference

| Variable | Role |
|----------|------|
| `VELOFIT_HOME` | Archive root (default `~/velofit`) |
| `SYMLOAD_DB` | SQLite path override |
| `SYMLOAD_USER` / `SYMLOAD_APP_PASSWORD` | IMAP |
| `SYMLOAD_IMAP_HOST` / `PORT` / `MAILBOX` | IMAP connection |
| `SYMLOAD_INGEST_VERBOSE` | Log skipped files on ingest |
| `POLAR_CLIENT_ID` / `POLAR_CLIENT_SECRET` | AccessLink OAuth client |
| `POLAR_REDIRECT_URI` | Default `http://127.0.0.1:8765/callback` |
| `POLAR_ACCESS_TOKEN` / `POLAR_USER_ID` | Usually from `polar_token.json` after auth |
| `POLAR_MEMBER_ID` | Register member-id (default `local-symload`) |

`email fetch`, `polar *`, and `sync` load `$VELOFIT_HOME/.env` when present; process env wins.

---

## Quick checklist

1. `db init` under `$VELOFIT_HOME`
2. Optional: IMAP + `email fetch` + `inbox promote`
3. Optional: Polar client + `polar auth` + `polar fetch`
4. Or: `sync --ftp <FTP>` with features `email,polar,sqlite`
5. `db status` / TUI Calendar `r`
6. `ftp set` history as thresholds change; `reprocess` when needed

For crate-level schema API details, see [../README.md](../README.md).
