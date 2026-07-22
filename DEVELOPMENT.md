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

1. **Feature development** → merge to `develop` (CI runs nightly `cargo fmt --check`, clippy, tests, TUI build, Python bindings).
2. **Staging / early access** → merge `develop` to `staging` (same CI gates). Beta releases (e.g. `0.2.0-beta.1`) can be published from here for testing.
3. **Release preparation**:
   - Create a branch `release/vX.Y.Z` from `staging`.
   - Update the version in the root `Cargo.toml` (the only place that needs changing because of `[workspace.package]`).
   - Update any relevant changelogs or release notes.
   - Open a PR from `release/vX.Y.Z` to `main`.
4. **Release**:
   - Merge the PR to `main`.
   - Create a git tag `vX.Y.Z`.
   - CI (triggered by the tag) runs the full release validation matrix (`release.yml`). Publishing to crates.io / PyPI is disabled until explicitly re-enabled.

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
- Pre-releases (e.g. `0.2.0-beta.1`, `0.2.0-rc.1`) are published from the `staging` branch.
- Final releases are cut from `release/vX.Y.Z` branches merged into `main` and tagged.

### CI / Release Automation Notes

- Every PR and push to `develop` / `staging` / `main` runs formatting (`cargo +nightly fmt --check` on library crates), clippy, library unit tests, and Python bindings (`maturin` + pytest) via [`.github/workflows/ci.yml`](.github/workflows/ci.yml). The TUI smoke build is currently **disabled** in CI (too slow / OpenBLAS-heavy); run `cargo build -p symworx-tui --bin symview` locally when touching the TUI.
- The release path is gated by [`.github/workflows/release.yml`](.github/workflows/release.yml):
  - **Checks only** on PRs into `main`, pushes to `main` / `release/**`, and tags `v*`:
    - Release metadata (workspace version matches `release/vX.Y.Z` branch name and/or tag; `CHANGELOG.md` has a `## [X.Y.Z]` section).
    - Nightly `cargo fmt --check`, a broader clippy + unit-test package set than day-to-day CI, and Python bindings (`maturin develop` + pytest + wheel smoke build).
  - **Publish is disabled** until a public release is approved. Scaffolded crates.io / PyPI / GitHub Release jobs live commented out at the bottom of `release.yml` (re-enable after trusted publishers and packaging are ready).
- Use of `cargo +nightly fmt -- --check` is required in the release workflow to enforce the full formatting rules defined in `rustfmt.toml`.
- Until `main` exists, validation still runs on `release/**` pushes and on tags (with a soft warning if the tag is not yet on `main`).

### Native / Heavy Dependencies

Several optional features pull in native libraries (notably OpenBLAS via `ndarray-linalg` for the `linalg` feature in `symworx-stats` and `symworx-core`).

- Default builds do **not** require these libraries.
- Full feature builds require the appropriate system development packages (see the crate READMEs for instructions).
- When publishing, we rely on feature flags so users only pull heavy dependencies when they explicitly opt in.

### Python Package Publishing

Higher-level Python packages are published independently on PyPI under the names:
- `symworx-biosym`
- `symworx-loadsym`
- `symworx-spatialsym`
- `symworx-core`

They are built from the corresponding `pyproject-*.toml` files using maturin. Trusted publishing (GitHub OIDC) is used so that no PyPI API tokens need to be stored as secrets.

### After a Release

- Verify that docs.rs has built the new versions with the correct feature sets.
- Update any external references (e.g. the personal starter guide) if the public API surface changed.
- Announce the release (if desired).

## Other Notes

- The `docs/` directory contains higher-level guides (e.g. the personal loadsym starter). These are not part of the generated `cargo doc` output.
- Internal agent guidelines live in `AGENTS.md`.
- Feel free to open issues or discussions for anything unclear in this document.

---

**Quick links**
- [AGENTS.md](AGENTS.md) – guidelines for agentic development
- [CONTRIBUTING.md](CONTRIBUTING.md)
- Root `Cargo.toml` for the full list of workspace members and shared dependencies
