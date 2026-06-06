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
