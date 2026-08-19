# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `symworx-math::circular` — wrap, circular mean, mean resultant length, circular SD
- `symworx-spatialsym` path linearity (`path_linearity`, `path_linearity_windows`) and pairwise effort / directional phase
- Spatial Visualize: plan-view canvas (auto-fit, pair edges, focused path vs chord) and A0–A1 effort-in strip with frame playhead
- Spatial TUI summary shows path efficiency / RMS deviation and pairwise effort / directional phase
- Windowed pairwise phase (`*_series` / `*_at`) and signed closing acceleration (`a_close`); TUI `Now` line uses a 1 s window
- `path_and_phase` example: straight vs weave, together / opposed / one-go-one-brake drills
- `symworx-dynamics::phase` — relative phase, Kuramoto order, cluster-phase (pre-extracted phases)

## [0.2.0] - 2026-08-07

### Added

- Linear mixed models in `symworx-stats` (`mixed` / `lmer`): random intercept and linear growth (REML/ML), design helpers, and simulators, behind the `linalg` feature

### Changed

- Workspace version bumped to 0.2.0
- Copyright holder updated to PalEm Dynamics LLC (Apache License 2.0 retained)

### Notes

- Python bindings still expose a **subset** of Rust stats (LMM not bound yet; expand in a later release).
- Welch PSD remains a placeholder; do not rely on it for analysis.
- Workspace remains a monorepo with a shared version and single changelog; this may change as this project grows.

## [0.1.1] - 2026-08-02

### Added

- `scripts/init-velofit.sh` — create `$VELOFIT_HOME` layout and empty LoadSym catalog for new users

### Changed

- Documentation pass: clearer crate READMEs, fewer redundant sections, aligned TUI keybinding notes
- Removed external product/book citations from docs and code comments

### Notes

- Python bindings still expose a **subset** of Rust stats (expand in a later release).
- Welch PSD remains a placeholder; do not rely on it for analysis.
- Workspace remains a monorepo with a shared version and single changelog; this may change as this project grows.

## [0.1.0] - 2026-07-31

### Added

- Initial release.

### Notes

- Python bindings still expose a **subset** of Rust stats (expand in a later release).
- Welch PSD remains a placeholder; do not rely on it for analysis.
- Workspace remains a monorepo with a shared version and single changelog; this may change as this project grows.

---

## Version Links

[0.2.0]: https://github.com/symworx/symworx/releases/tag/v0.2.0
[0.1.1]: https://github.com/symworx/symworx/releases/tag/v0.1.1
[0.1.0]: https://github.com/symworx/symworx/releases/tag/v0.1.0
