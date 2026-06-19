# SymWorx

**SymWorx** is a multi‑use computational framework designed for broad applicability across embedded systems, scientific computing, web applications, and educational environments.
At its core, **SymWorx** provides a **Rust kernel** with **Python bindings**, ensuring consistency, safety, and performance across platforms.

**SymWorx** is an open source platform that provides an isolated environment for modeling, analysis, and simulation. 

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
This project is licensed under the [Mozilla Public License, version 2.0](https://www.mozilla.org/media/MPL/2.0/index.f75d2927d3c1.txt).

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

**Overview**: The `symworx-loadsym` crate contains resources for quantifying and optimizing (exercise programming) training load (physiological and mechanical).
It also contains resources centered around nutrition and energy 
(e.g., basal metabolic rate, total daily energy expenditure, etc.)

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
- No other crate (including the TUI or analysis code) should bypass it by depending directly on `parquet`, `polars/parquet`, or similar for reading/writing signal data. This avoids pulling duplicate and conflicting low-level stacks (Arrow, brotli, zstd, allocators, etc.).
- Optional heavy analysis libraries (e.g. Polars) may be used for in-memory work, but data must first be loaded through `symworx-io`.
- The TUI follows this rule: it depends on `symworx-io` for loading and keeps any Polars usage strictly for in-memory exploration (never for I/O).


