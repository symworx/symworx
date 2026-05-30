# AGENTS.md — SymWorx Development Guidelines

This file contains instructions for AI coding agents (Grok, Claude, etc.) working in this repository.

## Project Overview

SymWorx is a Rust workspace focused on **biosignal analysis** and **nonlinear dynamics** (especially Recurrence Quantification Analysis / RQA).

Key crates:
- `symworx-biosym` — Physiological signal generators (PPG, respiration, CPG models, etc.)
- `symworx-io` — Loading/saving signals (CSV, Parquet, IBI, etc.)
- `symworx-dynamics` — Embedding, entropy, RQA/CRQA (core algorithms)
- `symworx-tui` — Terminal UI (`symview`) — current primary focus
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
- `Ctrl+R` or `F5` → Refresh file list (must be reliable even while typing)
- `Ctrl+Left` / `Ctrl+Right` → Tab navigation (kept for convenience)
- Bare `Ctrl+1/2/3` still exist but are **not** the primary method
- `q` → Hard quit (always works)
- `Esc` → Cancel current sub-mode (column picker, generate menu, filter) **or** quit if nothing active
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
- In `symworx-stats`, SVD, PCA, and the high-quality `l2`/`l1` regression implementations live behind an opt-in **`linalg`** feature.
  - The feature is **off by default** in the standalone `symworx-stats` crate. This keeps the dependency footprint small for common use cases (basic statistics, variability metrics, correlations, etc.).
  - `symworx-core` enables the `linalg` feature on `symworx-stats` by default for convenience, since most consumers go through `symworx-core`.
- Do **not** add `ndarray-linalg` (or similar) unconditionally to `symworx-math`, `symworx-stats`, or leaf crates unless the functionality is core to that crate *and* cannot reasonably be feature-gated.
- When a heavy dependency is truly required, prefer feature-gating it and documenting the cost (compile time, binary size, native requirements).

### General Rules
- Prefer pure-Rust or already-present workspace dependencies when possible.
- `symworx-math` is the canonical home for low-level numerical and sequence primitives.
- `symworx-stats` owns statistical descriptors and light modeling on top of those primitives.
- If you need to add a new dependency, follow the "When to Ask vs. When to Just Do It" rule above.

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

**Last updated:** After introducing the `linalg` feature in `symworx-stats` (opt-in by default in the crate, enabled via `symworx-core`) and documenting dependency hygiene principles.

When you start a new session, read this file and respect the TUI input priority rules above.
