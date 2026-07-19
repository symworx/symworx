# SymWorx

**SymWorx** is a multi‑use computational framework designed for broad applicability across embedded systems, scientific computing, web applications, and educational environments.
At its core, **SymWorx** provides a **Rust kernel** with **Python bindings**, ensuring consistency, safety, and performance across platforms.

**SymWorx** is an open source platform that provides an isolated environment for modeling, analysis, and simulation. 
│  A0: (  0.0,  0.0)  CL:Expansion    conf=0.30  spd=0.0  fwd=+0.00  ball=N  near=1.0  free=2.7  dfoc=4.5                                                                                                       │
│  A1: (  3.6,  2.5)  CL:Pressure     conf=0.62  spd=4.0  fwd=+1.00  ball=Y  near=3.7  free=4.0  v2f=+3.79  dfoc=0.2                                                                                            │
│  A2: (  1.0, -0.1)  CL:Denial       conf=0.70  spd=5.2  fwd=-0.64  ball=N  near=1.0  free=2.3  v2f=+5.17  dfoc=3.8  
## Philosophy

Our philosophy emphasizes:

i) **security** by minimizing unafe code, reducing unintended execution paths, and lowering supply‑chain risk,

ii) **robustness** through predictable behavior, strong typing, and explicit error handling, and 

iii) **scalability** via consistent APIs across embedded, desktop, and cloud environments.

Much of the original work was developed in Python, but is now being rewritten in Rust.
This shift was motivated by both a desire to deeply learn Rust and the opportunity to build a portable, high‑assurance computational engine that can 
i) run on microcontrollers and bare‑metal systems,
ii) integrate seamlessly with Python for education, data science, and rapid prototyping,
iii) serve as a stable foundation for higher‑level simulation frameworks

**License:**
This project is licensed under the [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0).

**Versioning Issues:**
If you encounter Python versioning issues, you can set environment variables to specify the Python version:
```
export PYO3_PYTHON=python3.12
export PYTHON_SYS_EXECUTABLE=python3.12
```

## Repository Structure

This **SymWorx** repository is a monorepo that contains a variety of `Rust` crates and `Python` packages; additional details can be found below.


### [Core](crates/symworx-core/README.md)

**Overview**: The `symworx-core` crate contains a variety of resources used across the subsequent simulation focused crates.
This includes backend resources, io, filters, processing, nonlinear dynamics, and statistics.


### [BioSym](crates/symworx-biosym/README.md)

**Overview**: The `symworx-biosym` crate contains modeling and simulation tools for biological signals and responses. 
Specifically, `symworx-biosym` contains physiological (ppg and respiratory) and biomechanical (gait). 
It also contains a central pattern generator (cpg) that integrates these signals.

#### RunSym (integrated)

**Note**: The former standalone `symworx-runsym` crate has been removed. Its functionality (modeling physiological and biomechanical responses to running, including runner/shoe interactions, fatigue, intensity, and performance simulation) is now being built out directly inside `symworx-biosym` under the biomechanics area (as discussed in the reorganization).

See `crates/symworx-biosym/src/` (particularly the `gait` module and future `running`-related modules under the `biomechanics` grouping) for the in-progress implementation. Legacy Python code lives at the external `~/worx/symworx/runsym` for reference during the port.


### [LoadSym](crates/symworx-loadsym/README.md)

**Overview**: The `symworx-loadsym` crate contains resources for quantifying and optimizing (exercise programming) training load (physiological and mechanical). Includes ACLi (acute-to-chronic load), monotony/strain, SEPi/TSLi/SRIi for power meter rides (.fit from SRM PC8, Garmin, Polar), plus nutrition (BMR/TDEE/weightloss).

TUI (symview) has a first-class **LoadSym** workflow (Home → 2) with Workout analysis, Calendar trends, and Optimization recommendations. Press `i`/`a` to load the newest `.fit` from `$VELOFIT_HOME` (default `~/velofit`) or `./data`.

See `crates/symworx-loadsym-db/docs/loadsym-personal-starter.md` for personal archive + SQLite catalog layout (data stays outside this repo).

### symload (headless)

```bash
cargo run -p symworx-loadsym --features "fit,email,sqlite" -- stats ride.fit --ftp 280
cargo run -p symworx-loadsym --features sqlite -- db init
cargo run -p symworx-loadsym --features sqlite -- ingest --ftp 280
```

Features: `fit`, `email`, `db` (print-schema), `sqlite` (init + ingest). Personal DB default: `$VELOFIT_HOME/db/loadsym.sqlite`.

### symworx-loadsym-db

Zero-dep SQL schema (Postgres + SQLite). No sample data. Consumable via the `db` / `sqlite` features of `symworx-loadsym`.

### Email / SRM ingestion

IMAP MIME extraction behind `email`. Credentials only via `SYMLOAD_USER` / `SYMLOAD_APP_PASSWORD` (or `$VELOFIT_HOME/.env`). Optional: `SYMLOAD_IMAP_HOST` / `PORT` / `MAILBOX` (defaults: Gmail `imap.gmail.com:993` / `INBOX`). Default drop zone: `$VELOFIT_HOME/inbox`. Host-side only — no AI/MCP dependency.

### [SpatialSym](crates/symworx-spatialsym/README.md)

**Overview**: The `symworx-spatialsym` crate provides sport-agnostic tools for 2D trajectory analysis and post-hoc interpretation of agent decision-making based on movement and use of space (expansion/penetration/denial/pressure). Analyses use historical and future context. Initial migration of legacy Python `spatialsystems` functionality with major fixes (no hardcoded FPS, proper vectors/atan2, typed data, etc.).

## Dependencies & Heavy/Optional Features

The workspace deliberately keeps heavy native dependencies minimal and opt-in where possible:

- `ndarray-linalg` (with OpenBLAS backend) is considered heavy (transitive cost: cauchy, LAPACK, native builds).
  - Gated behind an opt-in `linalg` feature in `symworx-stats` (off by default in the crate; `symworx-core` enables it for most users).
  - `symworx-signal` has a **direct unconditional dependency** because advanced linear algebra (deconvolution, NNLS, etc.) is core to its signal processing functionality.
- See `AGENTS.md` ("Crate Responsibilities & Dependency Hygiene") and the comments in the root `Cargo.toml` for the full rationale and rules.
- Workspace `Cargo.toml` pins `ndarray-linalg = { ..., features = ["openblas"] }` and the matching `openblas-build` build dependency.

### I/O Principle
- `symworx-io` is the canonical layer for all signal file I/O (Parquet, CSV, IBI, etc.).
- No other crate (including the TUI or analysis code) should bypass it by depending directly on `parquet`, `polars/parquet`, or similar for reading/writing signal data. This avoids pulling duplicate and incompatible transitive stacks (e.g. multiple versions of Arrow, brotli, zstd, and their allocator crates).
- Keep the core I/O layer stable and independent of heavy optional analysis libraries.
- The TUI follows this rule: it depends on `symworx-io` for loading and keeps any Polars usage strictly for in-memory exploration (never for I/O).


