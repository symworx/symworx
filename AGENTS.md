# AGENTS.md — SymWorx Development Guidelines

This file contains instructions and guidelines for agentic development tools working in this repository.

All changes must be owned by the human contributor, who is responsible for reviewing, explaining, and maintaining the code.

## Project Overview

SymWorx is a Rust workspace focused on **biosignal analysis** and **nonlinear dynamics** (especially Recurrence Quantification Analysis / RQA).

Key crates:
- `symworx-biosym` — Physiological signal generators (PPG, respiration, CPG models, etc.)
- `symworx-io` — Loading/saving signals (CSV, Parquet, IBI, etc.)
- `symworx-dynamics` — Embedding, entropy, RQA/CRQA (core algorithms)
- `symworx-tui` — Terminal UI (`symview`) — current primary focus
- `symworx-embed` — Host-side live streaming (PPG JSON protocol, serial/simulator sources, ring buffers); not firmware
- `symworx-math` — Low-level numerical and sequence primitives (including the canonical home for series operations in `src/series.rs`)
- `symworx-stats`, `symworx-backend`, Python bindings, etc.

## Development Focus

The main active work is on **`crates/symworx-tui`** (the `symview` TUI).

### Tab Architecture (agreed)
- **Import** (Tab 1): File discovery, CSV header handling, conversion, demo data generation (`Ctrl+G`)
- **Explore** (Tab 2): Statistics, simple visualization (Sparkline), future filtering/processing/edim/fnn
- **Dynamics** (Tab 3): RQA, recurrence plots, nonlinear tools (future)

### Key TUI Conventions (Important — Do Not Break)

**Keybindings (finalized after multiple iterations):**
- `Ctrl+G` → Open Generate demo data menu (in Import tab)
- `Ctrl+L` → Start/restart **live simulator** stream (Explore; host path via `symworx-embed`, `sid=S001`). Bare `l`/`L` remains Explore pan — do not reuse.
- `Ctrl+R` or `F5` → Refresh file list (must be reliable even while typing)
- `Ctrl+Left` / `Ctrl+Right` → Tab navigation (kept for convenience)
- Bare `Ctrl+1/2/3` still exist but are **not** the primary method
- `q` → Hard quit (always works)
- `Esc` → Stop live stream if active · cancel current sub-mode (column picker, generate menu, filter) **or** quit if nothing active
- In Import tab: `/` enters filter mode, `c` converts selected file

**Critical Implementation Rules for TUI:**
- **Input priority is extremely important.** Sub-modes (`pending_generate`, `pending_load`, `filter_mode`) **must** be checked **before** generic character handlers (especially the `manual_path.push(c)` arm).
- When a modal/overlay is active (`pending_generate` or column selection), most keys should be swallowed (`return false`) so they do not leak into `manual_path` or `file_filter`.
- Refresh (`Ctrl+R` / `F5`) should be handled early with an early return.
- After generating data with `Ctrl+G`, we clear `manual_path` and `file_filter`, refresh the file list, and load the signal directly into Explore.

**Demo Data Generation:**
- Uses `symworx-biosym` under the hood.
- Generated files **must** include headers (user requirement for column naming + future time-axis graphing).
- After generation, load the *signal* column (usually column index 1), not the time column.

## Development Commands

```bash
# Run the TUI
cargo run -p symworx-tui --bin symview

# Check only the TUI (fast)
cargo check -p symworx-tui

# Full workspace check
cargo check --workspace

# Run with a specific example
cargo run -p symworx-tui --example generate_biosym_demo
```

## Working Style Preferences

- **Prefer incremental, working changes.** Get something visible and useful quickly, then iterate based on feedback.
- The user has strong opinions about **UX and keybinding ergonomics** in the TUI. When in doubt, ask before changing key behavior.
- Keep the TUI keyboard-driven and mode-aware (like a mini vim/emacs experience).
- Avoid over-engineering early. The Explore tab currently has a working Sparkline + stats — that's the current "simple visualization" baseline.
- When editing `main.rs` in the TUI, be extremely careful with the ordering inside `handle_key` and `handle_import_keys`.

## When to Ask vs. When to Just Do It

**Ask first when:**
- Changing keybindings or input priority
- Major restructuring of App state or tab rendering
- Adding new dependencies
- The user has previously expressed strong preferences on the topic

**Just implement when:**
- Bug fixes that match previous explicit intent
- Straightforward visualization or stats improvements
- Small refactors that don't change behavior

## File Locations of Note

- TUI entrypoint + all logic: `crates/symworx-tui/src/main.rs` (large but manageable)
- Demo generation: `crates/symworx-tui/src/generate.rs`
- Conversion logic: `crates/symworx-tui/src/convert.rs`
- RQA core (already implemented): `crates/symworx-dynamics/src/rqa/`
- Low-level series / sequence primitives (canonical implementation): `crates/symworx-math/src/series.rs`

**Important:** Successive difference logic and other general sequence operations belong in `symworx-math`, **not** in `symworx-stats`, `symworx-signal`, or domain crates like `symworx-biosym`. Re-use via `symworx-core::math::series` (or direct `symworx-math`).

## Crate Responsibilities & Dependency Hygiene

The workspace strongly prefers **minimal, intentional dependencies**. Every new dependency (especially those bringing native code, large transitive graphs, or platform-specific build requirements) must be justified.

### Linear Algebra & Heavy Dependencies
- `ndarray-linalg` (and its transitive dependencies such as `cauchy`, `lax`, and LAPACK backends like OpenBLAS) is considered **heavy**.
- `polars` (and its transitive dependencies: arrow, multiple compression crates, etc.) is also heavy. It is centralized in the workspace for version consistency because we may want/need it in more than just the TUI in the future. Keep usage behind opt-in features in consuming crates.
- In `symworx-stats`, SVD, PCA, and closed-form regression (`l2`/`ols`/`ridge`) live behind an opt-in **`linalg`** feature.
  - Lasso / Elastic Net (coordinate descent), k-means clustering, and nonlinear least squares do **not** require `linalg`.
  - The feature is **off by default** in the standalone `symworx-stats` crate. This keeps the dependency footprint small for common use cases (basic statistics, variability metrics, correlations, etc.).
  - `symworx-core` enables the `linalg` feature on `symworx-stats` by default for convenience, since most consumers go through `symworx-core`.
- Optimization primitives (gradient descent, finite differences) live in **`symworx-math::optimize`** — pure Rust, no LAPACK.
- `symworx-signal` has a **direct unconditional dependency** on `ndarray-linalg` (via the workspace definition). This is intentional: advanced linear algebra (e.g. deconvolution, NNLS/Weiner solvers in `processing/deconvolution/`) is core to the crate and cannot reasonably be feature-gated for its primary use cases.
- The workspace `Cargo.toml` declares `ndarray-linalg` with `features = ["openblas"]` (and the corresponding `openblas-build` build-dep). Do **not** add `ndarray-linalg` (or similar) unconditionally to `symworx-math`, `symworx-stats`, or most leaf crates unless the functionality is core *and* cannot reasonably be feature-gated.
- When a heavy dependency is truly required, prefer feature-gating it (as done in stats) and documenting the cost (compile time, binary size, native OpenBLAS requirements). Signal is the explicit exception noted above.

### General Rules
- Prefer pure-Rust or already-present workspace dependencies when possible.
- `symworx-math` is the canonical home for low-level numerical, sequence, and optimization primitives.
- `symworx-stats` owns statistical descriptors and classical modeling (regression, PCA/SVD, clustering) on top of those primitives.
- **`symworx-embed`** owns host-side device streaming and framing (JSON-line PPG protocol, serial/simulator sources, rolling buffers, simple vitals thresholds).
  - Use **subject** terminology: wire/API field **`sid`** (subject id). Accept legacy `patient_id` on ingress only for SentryWard compatibility; never emit `patient_id` outbound.
  - Feature-gate hardware deps (`serial`); keep default path light (`simulate` only).
  - Do **not** put Embassy / `no_std` firmware or heavy LA/polars in this crate’s default build. Firmware (Arduino today, Embassy later) stays separate.
  - Recording streams to disk still goes through **`symworx-io`**. Signal algorithms (peak detect, filters) stay in **`symworx-signal`**.
- Data-driven dynamical operators (DMD, Koopman, SINDy) belong in **`symworx-dynamics`**, not stats.
  - DMD: `symworx-dynamics::dmd` (uses `symworx-stats` SVD via `linalg`).
  - EDMD / Koopman: `symworx-dynamics::koopman`.
  - SINDy (STLS): `symworx-dynamics::sindy` (polynomial library + sparse regression).
  - SINDYc (with control): `symworx-dynamics::sindyc` — library `Θ(x,u)`, forced simulation.
  - LTI plants + PID + state feedback: `symworx-dynamics::control`.
- Sparse sensing / compressed sensing reconstruction belongs in **`symworx-signal`**, not stats.
  - ISTA / OMP / sensing matrices: `symworx-signal::processing::sparse_sensing`.
- State estimation stays in **`symworx-signal`** (avoids dynamics↔signal cycles via `symworx-core`):
  - Linear Kalman + RTS + LTI constructors: `filters::nonlinear::kalman`
  - EKF: `filters::nonlinear::ekf`
  - UKF: `filters::nonlinear::ukf`
- If you need to add a new dependency, follow the "When to Ask vs. When to Just Do It" rule above.

### I/O Layer (Critical Rule)
- `symworx-io` is the **single source of truth** for all on-disk signal formats and I/O (CSV, Parquet with the project's chosen compression settings, IBI, etc.).
- Every other crate — including `symworx-tui`, analysis code, generators, and future tools — **must** load and save files through `symworx-io` (its readers, writers, or the traits it defines). Do **not** depend directly on `parquet`, `polars/parquet`, or other libraries' file I/O for signal data.
- This rule exists to:
  - Guarantee consistent format support and compression behavior across the workspace.
  - Prevent pulling in duplicate and incompatible transitive stacks (e.g. multiple versions of Arrow, brotli, zstd, and their allocator crates).
  - Keep the core I/O layer stable and independent of heavy optional analysis libraries.
- Analysis libraries such as (optional) Polars may be used for in-memory DataFrames, lazy queries, or exploration — **but only after data has been loaded via `symworx-io`**. Conversion adapters (e.g. Arrow RecordBatches or iterators → Polars) are acceptable.
- See the TUI as the canonical example: it depends on `symworx-io` for loading and may optionally use Polars only for in-memory work. The `polars-parquet` (or equivalent) feature must never be enabled for I/O purposes.

### Unit Conventions (Body Measurements)
- **Human body linear dimensions** (height, leg length, step length, stride length, etc.) are standardized to **meters** across the workspace.
  - `symworx-biosym` (gait parameters) uses meters.
  - `symworx-loadsym` nutrition functions (`calculate_bmr`, `calculate_weightloss`, `calculate_bmi`) accept height in **meters** (`height_m`).
- Legacy equations that internally require different units (e.g. Mifflin-St Jeor BMR expects cm) must perform the conversion **inside the function**.
- Mass is consistently expressed in **kilograms (kg)** everywhere.
- This convention was adopted to reduce cross-crate bugs and follow SI/biomechanics norms. See the 2026 loadsym height migration for rationale.

## Python Bindings

RQA and RecurrencePlot are exposed via PyO3 in `bindings/python/`. Keep the Rust side as the source of truth.

---

**Last updated:** Host-first `symworx-embed` (SentryWard concepts: JSON PPG protocol, `sid` subject naming, serial/simulator sources). Still covers the core I/O rule (`symworx-io` only for on-disk signal I/O), TUI input priority, and `ndarray-linalg` / polars hygiene.

When you start a new session, read this file and respect the TUI input priority rules above.
