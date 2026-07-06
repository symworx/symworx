# SymWorx-LoadSym

**SymWorx-LoadSym** provides the tools and resources for load and monitoring based calculations and estimations.

## Quick Start (Rust)

```rust
use symworx_loadsym::load::{compute_acute_chronic, classify_acwr, compute_monotony, RiskLevel};

let daily_loads: Vec<f64> = (0..30).map(|i| 400.0 + (i as f64 % 7.0) * 30.0).collect();

let snap = compute_acute_chronic(&daily_loads, 7, 28).unwrap();
println!("ACWR = {:.2} → {:?}", snap.acwr, snap.risk_level);

let mono = compute_monotony(&daily_loads[23..]).unwrap();
println!("Recent monotony: {:.2}", mono);
```

## Python (via maturin)

```bash
cd bindings/python
maturin develop --manifest-path ../crates/symworx-loadsym/Cargo.toml -m pyproject-loadsym.toml
```

```python
import symworx_loadsym as loadsym

loads = [400.0 + (i % 7) * 30 for i in range(30)]
acute, chronic, acwr, risk = loadsym.load.compute_acute_chronic(loads, 7, 28)
print(acwr, risk)

print(loadsym.load.classify_acwr(1.6))  # "High"
```

See `src/load/acwr.rs` and `src/load/monotony.rs` for the full API and sports-science rationale.
The rolling/EWMA primitives live in `symworx-math` (re-exported via `symworx-core`).

## CLI (symload)

The `symworx-loadsym` crate ships a binary called `symload` for headless use:

```bash
# Stats + metrics
cargo run -p symworx-loadsym --features "email,db" -- stats /path/to/ride.fit --ftp 280 --json

# DB schema
cargo run -p symworx-loadsym --features db -- db print-schema

# Email fetch (SRM etc.)
cargo run -p symworx-loadsym --features email -- email fetch ~/symload/inbox
```

After `cargo install symworx-loadsym --features "email,db"` the `symload` command is available globally.

See `crates/symworx-loadsym-db/docs/loadsym-personal-starter.md` for recommended layout (`~/symload/inbox`, etc.) and integration with a separate DB project.

The email fetching logic lives in `symworx-io` (under the `email` feature) because it is an I/O source.

## Power Ride Metrics (new)

```rust
use symworx_loadsym::load::compute_ride_metrics;
let m = compute_ride_metrics(&times_s, &power_w, ftp_w = 300.0);
println!("NP={} TSS={:.1}", m.np, m.tss);
```

Useful for SRM PC8 / Garmin / Polar .fit files imported via `symworx-io`. TUI LoadSym uses these for Workout summaries.

