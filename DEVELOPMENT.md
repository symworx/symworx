# Development

This document describes how to build, test, and contribute to the SymWorx workspace.

For guidelines when working with AI/agentic development tools, see [AGENTS.md](AGENTS.md). All contributors are expected to take full ownership of submitted code.

## Prerequisites

- Rust (stable + nightly toolchain recommended for formatting)
- Cargo
- For full native linear algebra features: OpenBLAS development headers (optional for most development)
- Python 3.12+ and maturin (for Python bindings development)
- (Optional) `cargo install cargo-release` or similar for local release management

## Common Commands

From the workspace root:

```bash
# Check / build
cargo check --workspace
cargo check -p symworx-tui
cargo build -p symworx-tui

# Run the TUI (symview)
cargo run -p symworx-tui --bin symview

# Personal LoadSym archive (empty dirs + catalog under ~/velofit)
./scripts/init-velofit.sh

# Test
cargo test --workspace

# Format (stable)
cargo fmt

# Format check with full unstable features (recommended in CI)
cargo +nightly fmt -- --check

# Clippy
cargo clippy --workspace --all-features -- -D warnings

# Generate docs
cargo doc --workspace --no-deps
```

Specific crates often have their own examples:

```bash
cargo run -p symworx-biosym --example gait_demo
cargo run -p symworx-loadsym --example nutrition_demo
cargo run -p symworx-spatialsym --example synthetic_decisions

# Recent data-driven / predictive stack
cargo run -p symworx-stats --example predictive_metrics_demo --features linalg
cargo run -p symworx-dynamics --example data_driven_dynamics_demo
cargo run -p symworx-signal --example sparse_sensing_demo
cargo run -p symworx-signal --example state_estimation_demo
cargo run -p symworx-signal --example windowed_rr_features
```

## Python Bindings Development

The Python bindings live in `bindings/python/`.

```bash
cd bindings/python
maturin develop --manifest-path ../crates/symworx-biosym/Cargo.toml -m pyproject-biosym.toml
# similarly for other pyproject-*.toml files
```

Run Python tests:

```bash
python -m pytest
```

See the individual `pyproject-*.toml` files for the higher-level packages (`symworx-biosym`, `symworx-loadsym`, `symworx-core`, etc.).

## Code Style

- Run `cargo fmt` before committing.
- In CI we run `cargo +nightly fmt -- --check` to catch formatting that depends on unstable options.
- Follow the lint rules defined in the root `Cargo.toml` (`[workspace.lints]`).
- Keep changes focused. One logical change per PR is preferred.

## Workspace Structure

This is a Cargo workspace. Low-level crates (e.g. `symworx-math`, `symworx-io`, `symworx-stats`) can be depended on individually. Grouping crates (`symworx-core`, `symworx-biosym`, `symworx-loadsym`, `symworx-spatialsym`, `symworx-tui`) provide higher-level APIs and re-exports.

All crates currently share a single version defined in the root `[workspace.package]`.

## Releasing

We follow a branch-based workflow with a single shared version across the workspace:

1. **Feature development** → merge to `develop` (day-to-day CI: fmt, clippy, lib tests, Python bindings).
2. **Stage / early access** → fast-forward `develop` → `stage` when you want a promotion point. Day-to-day CI does **not** run on `stage` (avoids double runs on FF); beta tags can still be cut from here if needed.
3. **Release preparation**:
   - Create a branch `release/vX.Y.Z` from `stage` (or from `develop` if stage is not updated yet).
   - Bump the shared version in the root `Cargo.toml`: `[workspace.package] version` **and** every internal crate `version = "…"` under `[workspace.dependencies]` (must stay in lockstep).
   - Update `CHANGELOG.md` (require a `## [X.Y.Z]` section for the release branch/tag).
   - Open a PR from `release/vX.Y.Z` to `main`.
4. **Release**:
   - Merge the PR to `main` when release checks are green.
   - **Manually** create and push the annotated tag `vX.Y.Z` on the merge commit (tags are not auto-created in CI).
   - Tag push runs [`.github/workflows/release.yml`](.github/workflows/release.yml): full validation, then **publish** to crates.io and create a **GitHub Release** page (only on `v*` tags after validation succeeds). **PyPI is paused** until `publish-python` is re-enabled.

### Publishing Order

Low-level crates must be published before crates that depend on them:

1. `symworx-error`
2. `symworx-math`
3. `symworx-io`, `symworx-stats`, `symworx-signal`, `symworx-dynamics`, `symworx-backend`, `symworx-embed`, `symworx-loadsym-db`
4. Grouping crates: `symworx-core`, `symworx-biosym`, `symworx-loadsym`, `symworx-spatialsym`, `symworx-tui`
5. Python packages (via the corresponding `pyproject-*.toml` files): `symworx-biosym`, `symworx-loadsym`, `symworx-spatialsym`, `symworx-core`

The `symview` binary (from `symworx-tui`) can be installed with `cargo install symworx-tui --bin symview` or distributed separately.

### Versioning

- We use a single version for the entire workspace.
- Semantic Versioning (SemVer) is followed.
- Pre-releases (e.g. `0.2.0-beta.1`, `0.2.0-rc.1`) may be tagged from `stage` or a release branch.
- Final releases are cut from `release/vX.Y.Z` branches merged into `main`, then **manually** tagged.

### CI / Release Automation Notes

- Day-to-day [`.github/workflows/ci.yml`](.github/workflows/ci.yml) runs on push/PR to **`develop`** only, plus **`workflow_dispatch`** (manual re-run of this gate; no publish). It formats library crates, clippy + unit-tests the **full science package set**, and builds Python bindings (`maturin develop` + pytest). The TUI smoke build is **disabled**; run `cargo build -p symworx-tui --bin symview` locally when touching the TUI.
- The release path is gated by [`.github/workflows/release.yml`](.github/workflows/release.yml):
  - **Validation** on PRs into `main`, pushes to `release/**`, tags `v*`, and `workflow_dispatch` (not on push to `main`; the PR already ran this workflow):
    - Release metadata (workspace version matches `release/vX.Y.Z` / tag; `CHANGELOG.md` has a `## [X.Y.Z]` section).
    - Same science fmt/clippy/test set as day-to-day CI, plus Python **wheel smoke** 
    - **Tags are created manually** after a green merge to `main` (for now, we do not have an auto-tag job).
  - **Publish** runs only on tag `v*` after validation succeeds:
    - crates.io
    - GitHub Release
    - **PyPI paused:** 

### Native / Heavy Dependencies

Several optional features pull in native libraries (notably OpenBLAS via `ndarray-linalg` for the `linalg` feature in `symworx-stats` and `symworx-core`).

- Default builds do **not** require these libraries.
- Full feature builds require the appropriate system development packages (see the crate READMEs for instructions).
- When publishing, we rely on feature flags so users only pull heavy dependencies when they explicitly opt in.

### Python Package Publishing

**Paused** in CI until a more stable build is ready. Validation still runs `maturin develop` / wheel smoke.

## Other Notes

- The `docs/` directory contains higher-level guides. [docs/projects.md](docs/projects.md) lists slated vs considered work. The LoadSym personal catalog starter **redirects** to the canonical copy under `crates/symworx-loadsym-db/docs/` (schema v4, multi-source ingest). These are not part of the generated `cargo doc` output.
- Internal agent guidelines live in `AGENTS.md`.
- Feel free to open issues or discussions for anything unclear in this document.

---

**Quick links**
- [AGENTS.md](AGENTS.md) – guidelines for agentic development
- [CONTRIBUTING.md](CONTRIBUTING.md)
- [Projects](docs/projects.md) – slated vs considered development items
- [LoadSym personal catalog](crates/symworx-loadsym-db/docs/loadsym-personal-starter.md) – `$VELOFIT_HOME`, ingest, Polar, sync
- [symworx-loadsym-db README](crates/symworx-loadsym-db/README.md) – schema versions / API
- Root `Cargo.toml` for the full list of workspace members and shared dependencies
