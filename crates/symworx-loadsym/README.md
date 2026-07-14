# SymWorx-LoadSym

**SymWorx-LoadSym** provides tools for training-load quantification and nutrition modeling.

## Quick Start (Rust)

```rust
use symworx_loadsym::load::{compute_acute_chronic, classify_acwr, compute_monotony};

let daily_loads: Vec<f64> = (0..30).map(|i| 400.0 + (i as f64 % 7.0) * 30.0).collect();

let snap = compute_acute_chronic(&daily_loads, 7, 28).unwrap();
println!("ACWR = {:.2} → {:?}", snap.acwr, snap.risk_level);

let mono = compute_monotony(&daily_loads[23..]).unwrap();
println!("Recent monotony: {:.2}", mono);
```

## Pulse-response (fitness–fatigue) + multi-day plans

Daily load series (e.g. TSS) drive a two-compartment model:

| Mode | Update | Interpretation |
|------|--------|----------------|
| **PMC** (`PulseResponseParams::pmc_defaults`) | EWMA CTL/ATL | `fitness`→CTL, `fatigue`→ATL, `form`→TSB |
| **Banister** (`banister_defaults`) | \(x_t = x_{t-1}e^{-1/\tau}+w_t\) | classic impulse-response; use \(k_h > k_g\) |

```rust
use symworx_loadsym::load::{
    LoadGoal, OptimizationThresholds, PulseResponseParams,
    optimize_load_plan, simulate_pulse_response,
};

let params = PulseResponseParams::pmc_defaults();
let series = simulate_pulse_response(&daily_loads, &params, None).unwrap();
let end = series.last_state().unwrap();
println!("CTL={:.0} ATL={:.0} TSB={:.0}", end.ctl(), end.atl(), end.tsb());

let thr = OptimizationThresholds { horizon_days: 3, ..Default::default() };
let plan = optimize_load_plan(&daily_loads, &params, LoadGoal::Recovery, &thr).unwrap();
println!("plan TSS {:?} success={}", plan.daily_tss, plan.success);
```

**Optimization goals** (default horizon 3 days):

| Goal | Intent | Success threshold (defaults) |
|------|--------|------------------------------|
| `Recovery` | Raise form, light days | ≥80% of rest-trajectory form gain |
| `Maintenance` | Hold form | Relative form drift ≤ **20%** |
| `Overload` | Controlled form dip + progressive load | Dip near target; ACWR hard cap 1.5 |

Search enumerates rest/easy/steady/hard/long templates scaled to chronic load (no external solver).

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
cargo run -p symworx-loadsym --features sqlite -- db status

# Schema only (no driver)
cargo run -p symworx-loadsym --features db -- db print-schema --sqlite

# Email fetch (credentials via env only — never commit)
# IMAP lives in symworx-io; CLI orchestrates drop → promote → ingest
export SYMLOAD_USER="you@example.com"
export SYMLOAD_APP_PASSWORD="your-app-password"
cargo run -p symworx-loadsym --features "fit,email" -- email fetch
# Optional custom IMAP SEARCH (default: SUBJECT SRM)
cargo run -p symworx-loadsym --features "fit,email" -- email fetch --query "OR SUBJECT SRM SUBJECT Polar"
cargo run -p symworx-loadsym --features fit -- inbox promote
```

### Features

| Feature | Purpose |
|---------|---------|
| `fit` | Load `.fit` for `stats` |
| `email` | IMAP fetch of `.fit` attachments (implies `fit`) |
| `db` | `db print-schema` from `symworx-loadsym-db` |
| `sqlite` | Personal catalog init + ingest (`rusqlite` + `fit` + `db`) |

### Environment

| Variable | Role |
|----------|------|
| `VELOFIT_HOME` | Archive root (default `~/velofit`) |
| `SYMLOAD_DB` | SQLite path override |
| `SYMLOAD_USER` / `SYMLOAD_APP_PASSWORD` | IMAP only |

**Privacy:** catalog + FIT files stay under `$VELOFIT_HOME`. Do not commit `*.sqlite`, `.env`, or ride archives into SymWorx.

See `crates/symworx-loadsym-db/docs/loadsym-personal-starter.md`.

## Power ride metrics

```rust
use symworx_loadsym::load::compute_ride_metrics;
let m = compute_ride_metrics(&times_s, &power_w, 300.0);
println!("NP={} TSS={:.1}", m.np, m.tss);
```
