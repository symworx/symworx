# SymWorx

**SymWorx** is an open-source computational stack for **biosignal analysis**, **training load**, **nonlinear dynamics**, and **classical ML** — with a **Rust kernel**, **Python bindings**, and a keyboard-driven TUI (`symview`).

It is aimed at research, education, and portable inference (workstation today; embedded/mobile recipes for exported models).

**License:** [Apache License 2.0](LICENSE)
**Version:** workspace `0.1.0` (monorepo; see [CHANGELOG.md](CHANGELOG.md))

---

## Quick start (by role)

### Desktop / biosignals (TUI)

```bash
# Optional: demo files already under ./data
cargo run -p symworx-tui --bin symview
```

In **Import**: browse `./data`, or `Ctrl+G` to generate demo signals. **Explore** for stats/sparkline; **Dynamics** for RQA.

### Classical ML / stats (Rust, no OpenBLAS required for many APIs)

```bash
cargo run -p symworx-stats --example logistic_regression_demo
cargo run -p symworx-stats --example multiclass_logistic_demo
cargo run -p symworx-stats --example rule_list_demo
cargo run -p symworx-stats --example train_test_split_demo
# OLS / LDA / polyreg need the linalg feature:
cargo run -p symworx-stats --example linear_regression_demo --features linalg
```

Model export (C / iOS / Android / web): [crates/symworx-stats/docs/model_export.md](crates/symworx-stats/docs/model_export.md)

### Training load (CLI)

```bash
cargo run -p symworx-loadsym --features "fit,sqlite" -- stats ride.fit --ftp 280
```

TUI: `symview` → Home → **LoadSym** (2). Personal catalog (schema v4, multi-source ingest): [crates/symworx-loadsym-db/docs/loadsym-personal-starter.md](crates/symworx-loadsym-db/docs/loadsym-personal-starter.md) · [crate README](crates/symworx-loadsym-db/README.md)

### Python (education / data science)

```bash
python -m venv .venv && source .venv/bin/activate
pip install "maturin>=1.5,<2.0" pytest
maturin develop --manifest-path bindings/python/Cargo.toml
pytest bindings/python/tests/ -q
```

```python
from symworx import biosym, loadsym, core
```

If PyO3 cannot find Python:

```bash
export PYO3_PYTHON=python3.12
export PYTHON_SYS_EXECUTABLE=python3.12
```

**Note (0.1):** Python exposes biosym, loadsym, and a growing **core.statistics** subset (splits, scaler, logistic binary/OVR, NB, metrics, ROC-AUC). Rule lists, k-NN, LDA, polyreg remain Rust-first for now.

---

## Who is this for?

| Audience | Start here |
|----------|------------|
| Educator / student | Stats examples + Python install |
| Biosignal researcher | `symview` + `symworx-signal` / dynamics |
| Coach / sport scientist | LoadSym TUI + `symload` |
| Spatial / team analyst | SpatialSym TUI workflow |
| Embedded / mobile / web | [model_export.md](crates/symworx-stats/docs/model_export.md) (train in Rust, infer elsewhere) |
| Contributor | [DEVELOPMENT.md](DEVELOPMENT.md), [AGENTS.md](AGENTS.md), [CONTRIBUTING.md](CONTRIBUTING.md) |

---

## Philosophy

1. **Security** — minimize unsafe code and reduce unintended execution paths  
2. **Robustness** — strong typing, explicit errors where the API is mature  
3. **Scalability** — one kernel for analysis, simulation, and portable inference  

Much of the original work lived in Python; the long-term engine is **Rust**, with Python for teaching and rapid prototyping.

---

## Supported vs experimental (0.1)

| Surface | Status |
|---------|--------|
| I/O via `symworx-io` (CSV, Parquet, IBI, activity/FIT) | **Supported** |
| Signal filters, peaks, many processing tools | **Supported** |
| RQA, embedding, entropy, DMD/SINDy (dynamics) | **Supported** |
| Stats: basic, splits, logistic (binary + OVR), NB, k-NN, rules, metrics, ROC/AUC | **Supported** (pure Rust paths) |
| Stats OLS/Ridge/PCA/SVD/LDA/polyreg (`linalg` + OpenBLAS) | **Supported** with native build deps |
| TUI Import / Explore / Dynamics (RQA) / LoadSym / Spatial | **Supported** (early UX) |
| Host embed streaming (`symworx-embed`) | **Supported** (simulate default) |
| Welch PSD (`stats::spectral`) | **Experimental** (placeholder) |
| CRQA full API | **Planned** |
| `symworx-backend` server | **Experimental** (stubs) |
| R bindings | **Stub** |
| Python modern ML API (logistic, splits, …) | **Partial** — expand for release |

---

## Repository structure (crates)

| Crate | Role |
|-------|------|
| [symworx-core](crates/symworx-core/README.md) | Re-exports + convenience |
| [symworx-math](crates/symworx-math/README.md) | Series, integrate, optimize, distributions |
| [symworx-stats](crates/symworx-stats/README.md) | Statistics + classical ML |
| [symworx-signal](crates/symworx-signal/README.md) | Filters, peaks, sparse sensing, Kalman |
| [symworx-dynamics](crates/symworx-dynamics/README.md) | RQA, embedding, DMD, SINDy, control |
| [symworx-io](crates/symworx-io/README.md) | **Canonical** signal file I/O |
| [symworx-biosym](crates/symworx-biosym/README.md) | PPG, respiration, gait, CPG |
| [symworx-loadsym](crates/symworx-loadsym/README.md) | ACWR, monotony, FIT, nutrition, `symload` (email/polar/sync) |
| [symworx-loadsym-db](crates/symworx-loadsym-db/README.md) | SQL schema only (v4 multi-source; no personal data) |
| [symworx-spatialsym](crates/symworx-spatialsym/README.md) | Trajectories, space metrics, decisions |
| [symworx-embed](crates/symworx-embed/README.md) | Host PPG streaming / simulator |
| [symworx-tui](crates/symworx-tui/README.md) | **`symview`** terminal UI |
| [symworx-backend](crates/symworx-backend/README.md) | Process/server utilities (early) |
| [symworx-error](crates/symworx-error/README.md) | Shared errors |
| [bindings/python](bindings/python/README.md) | PyO3 package `symworx` |

Former **RunSym** lives inside **biosym** (gait / run performance), not as a separate crate.

---

## Dependencies & heavy features

- **`ndarray-linalg` / OpenBLAS** — heavy. Opt-in on `symworx-stats` via `features = ["linalg"]`. **Unconditional** on `symworx-signal` (deconvolution, etc.).
- **`polars`** — optional in TUI for in-memory frames only; **never** for signal file I/O.
- **I/O rule:** all on-disk signal formats go through **`symworx-io`** only.

Details: [AGENTS.md](AGENTS.md), [DEVELOPMENT.md](DEVELOPMENT.md).

### System deps (when using `linalg` or signal LA)

Linux (Fedora/Debian-style): install OpenBLAS development packages (CI uses system OpenBLAS).  
Without OpenBLAS, prefer pure-Rust stats examples (logistic, rules, splits, k-NN, NB).

---

## Development

```bash
cargo check --workspace
cargo test -p symworx-stats
cargo +nightly fmt -- --check
```

See [DEVELOPMENT.md](DEVELOPMENT.md) and [CONTRIBUTING.md](CONTRIBUTING.md).

Agent / AI contributors: read [AGENTS.md](AGENTS.md) and own all submitted code.

---

## Documentation map

| Doc | Content |
|-----|---------|
| [DEVELOPMENT.md](DEVELOPMENT.md) | Build, test, format |
| [AGENTS.md](AGENTS.md) | Crate boundaries, TUI keys, dependency hygiene |
| [crates/symworx-stats/docs/model_export.md](crates/symworx-stats/docs/model_export.md) | Export models to C / iOS / Android / web |
| [publications/joss/2026/](publications/joss/2026/) | JOSS paper draft + tracking notes |
