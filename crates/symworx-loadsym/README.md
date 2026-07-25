# SymWorx-LoadSym

**SymWorx-LoadSym** provides tools for training-load quantification and nutrition modeling.

## Quick Start (Rust)

```rust
use symworx_loadsym::load::{compute_acute_chronic, classify_acwr, compute_monotony};

let daily_loads: Vec<f64> = (0..30).map(|i| 400.0 + (i as f64 % 7.0) * 30.0).collect();

let snap = compute_acute_chronic(&daily_loads, 7, 28).unwrap();
println!("ACLi = {:.2} → {:?}", snap.acwr, snap.risk_level);

let mono = compute_monotony(&daily_loads[23..]).unwrap();
println!("Recent monotony: {:.2}", mono);
```

## Pulse-response (fitness–fatigue) + multi-day plans

Daily load series (e.g. TSLi) drive a two-compartment model:

| Mode | Update | Interpretation |
|------|--------|----------------|
| **PMC** (`PulseResponseParams::pmc_defaults`) | EWMA LTSLi/STSLi | `fitness`→LTSLi, `fatigue`→STSLi, `readiness`→SLBi |
| **Banister** (`banister_defaults`) | \(x_t = x_{t-1}e^{-1/\tau}+w_t\) | classic impulse-response; use \(k_h > k_g\) |

```rust
use symworx_loadsym::load::{
    LoadGoal, OptimizationThresholds, PulseResponseParams,
    optimize_load_plan, simulate_pulse_response,
};

let params = PulseResponseParams::pmc_defaults();
let series = simulate_pulse_response(&daily_loads, &params, None).unwrap();
let end = series.last_state().unwrap();
println!("LTSLi={:.0} STSLi={:.0} SLBi={:.0}", end.ctl(), end.atl(), end.tsb());

let thr = OptimizationThresholds { horizon_days: 3, ..Default::default() };
let plan = optimize_load_plan(&daily_loads, &params, LoadGoal::Recovery, &thr).unwrap();
println!("plan TSLi {:?} success={}", plan.daily_tss, plan.success);
```

**Optimization goals** (default horizon **4** days, max **10**) — primary success is **mean planned load vs chronic mean \(C\)** (last ≤28 days of TSLi):

| Goal | Intent | Success (defaults) | Scoring prefers |
|------|--------|--------------------|-----------------|
| `Recovery` | Active recovery | \(0.20\,C \le \bar w \le 0.55\,C\) | ~0.38·C days (not pure rest) |
| `Maintenance` | Hold load | \(0.85\,C \le \bar w \le 1.15\,C\) | Mean near \(C\) **with day-to-day TSLi variance** (not flat) |
| `Overload` | Elevated load | \(1.15\,C \le \bar w \le 1.40\,C\), \(\bar w > C\) | ~1.25·C with variety; soft limit consecutive hard days |

Readiness (SLBi) and projected ACLi are **soft / separate context** — they do not alone hard-fail success.

Search uses a fine template grid (rest → long). Short horizons fully enumerate
candidates; longer horizons (when `7^H` is large) use **beam search** so H=10 stays interactive.

Demo:

```bash
cargo run -p symworx-loadsym --example pulse_response_demo
```

## CLI (`symload`)

```bash
# Stats on a FIT file
cargo run -p symworx-loadsym --features fit -- stats path/to/ride.fit --ftp 280

# Personal SQLite catalog (file lives under $VELOFIT_HOME — not in this repo)
export VELOFIT_HOME="$HOME/velofit"   # optional default
cargo run -p symworx-loadsym --features sqlite -- db init
cargo run -p symworx-loadsym --features sqlite -- ingest --ftp 280
# Only files with mtime >= last_ingest_at (watermark in catalog_meta). Full recheck:
cargo run -p symworx-loadsym --features sqlite -- ingest --all --ftp 280
# Re-score everything (ignores watermark + hash skip):
cargo run -p symworx-loadsym --features sqlite -- ingest --force --ftp 280
# Multi-source dedup: keep all copies, count one (power-meter preferred)
cargo run -p symworx-loadsym --features sqlite -- relink
cargo run -p symworx-loadsym --features sqlite -- db status

# Schema only (no driver)
cargo run -p symworx-loadsym --features db -- db print-schema --sqlite

# Email fetch (credentials via env or $VELOFIT_HOME/.env — never commit)
# IMAP lives in symworx-io; CLI orchestrates drop → promote → ingest
# Host-side only: no AI/MCP dependency
export SYMLOAD_USER="you@example.com"
export SYMLOAD_APP_PASSWORD="your-app-password"
# Optional host (default imap.gmail.com):
# export SYMLOAD_IMAP_HOST=outlook.office365.com
cargo run -p symworx-loadsym --features "fit,email" -- email fetch
# Optional custom IMAP SEARCH (default: SUBJECT SRM)
cargo run -p symworx-loadsym --features "fit,email" -- email fetch --query "OR SUBJECT SRM SUBJECT Polar"
cargo run -p symworx-loadsym --features fit -- inbox promote

# Polar AccessLink (client id/secret in $VELOFIT_HOME/.env)
cargo run -p symworx-loadsym --features polar -- polar auth
cargo run -p symworx-loadsym --features polar -- polar fetch
cargo run -p symworx-loadsym --features sqlite -- ingest --ftp 280

# Unified multi-source (build with the features you need):
cargo run -p symworx-loadsym --features "email,polar,sqlite" -- sync --ftp 280
```

### Features

| Feature | Purpose |
|---------|---------|
| `fit` | Load `.fit` for `stats` |
| `email` | IMAP fetch of `.fit` attachments (implies `fit`) |
| `polar` | Polar AccessLink OAuth + exercise FIT download (implies `fit`) |
| `db` | `db print-schema` from `symworx-loadsym-db` |
| `sqlite` | Personal catalog init + ingest (`rusqlite` + `fit` + `db`) |

### Environment

| Variable | Role |
|----------|------|
| `VELOFIT_HOME` | Archive root (default `~/velofit`) |
| `SYMLOAD_DB` | SQLite path override |
| `SYMLOAD_USER` / `SYMLOAD_APP_PASSWORD` | IMAP credentials |
| `SYMLOAD_IMAP_HOST` | IMAP host (default `imap.gmail.com`) |
| `SYMLOAD_IMAP_PORT` | IMAP TLS port (default `993`) |
| `SYMLOAD_IMAP_MAILBOX` | Mailbox to search (default `INBOX`) |
| `POLAR_CLIENT_ID` / `POLAR_CLIENT_SECRET` | AccessLink OAuth client |
| `POLAR_REDIRECT_URI` | Default `http://127.0.0.1:8765/callback` |
| `POLAR_ACCESS_TOKEN` / `POLAR_USER_ID` | Usually from `polar_token.json` after `polar auth` |

`email fetch` and `polar *` also load `$VELOFIT_HOME/.env` when present (process env wins).

**Privacy:** catalog + FIT files stay under `$VELOFIT_HOME`. Do not commit `*.sqlite`, `.env`, `polar_token.json`, or ride archives into SymWorx.

### Personal catalog (schema)

Schema-only crate: **`symworx-loadsym-db`** (current **SCHEMA_VERSION = 4**).

- Crate overview: [../symworx-loadsym-db/README.md](../symworx-loadsym-db/README.md)
- Operator guide (layout, multi-source, Polar, sync, timers): [../symworx-loadsym-db/docs/loadsym-personal-starter.md](../symworx-loadsym-db/docs/loadsym-personal-starter.md)

Multi-source sessions keep all FIT copies but only **`counts_for_load = 1`** rows feed `daily_loads` / ACWR / PMC. Prefer power-meter / email over Polar when both match (`relink` / end of `ingest`).

## Power ride metrics

```rust
use symworx_loadsym::load::compute_ride_metrics;
let m = compute_ride_metrics(&times_s, &power_w, 300.0);
println!("SEPi={} TSLi={:.1}", m.np, m.tss);
```
