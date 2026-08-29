# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [new changes]

### Notes

- Transfer entropy is a discrete quantile-binned estimator on `feature/transfer-entropy`. Not yet bound in Python or `symview`.
- Shared relative discretization (grouped k-means cuts) lives on `feature/discretize-pipeline`. HRV/sleep is an example use, not a typed API.

### Added

- `symworx-dynamics` discrete transfer entropy: `transfer_entropy` / `transfer_entropy_with` (bivariate), `transfer_entropy_mv` (joint sources), `transfer_entropy_conditional` (partial TE), plus `TeConfig`
- `symworx-math::series::discretize` — apply interior cuts to a series (`u8` bins)
- `symworx-stats::discretize` — `GroupedRangeScaler` + `RelativeKMeansDiscretizer` (per-user range, pooled k-means cuts on `[0, 1]`)
- `symworx-stats::grouped_train_test_split` — hold out whole groups (users)
- `transfer_entropy_discrete` / `_mv` / `_conditional` — TE on pre-binned labels (sleep stages, relative HRV bins); `TeConfig::bins` ignored
- `relative_te_demo` example — synthetic multi-user relative bins + per-night TE

## [0.3.2] - 2026-08-22

Same contents as 0.3.0. crates.io republish after a partial 0.3.1 upload.

### Notes

- `v0.3.0` GitHub tag did not publish.
- `0.3.1` was uploaded for most crates from a local `cargo publish` on develop; That number is burned.
- All workspace crates are now 0.3.2

## [0.3.0] - 2026-08-22

### Added

- Python `symworx.core.statistics.lmer` / `MixedModel` and `simulate_random_intercept` (random intercept or linear growth)
- `symworx-math::circular` — wrap, circular mean, mean resultant length, circular SD
- `symworx-spatialsym` path linearity (`path_linearity`, `path_linearity_windows`) and pairwise effort / directional phase
- Spatial Visualize: plan-view canvas (pair edges, focused path vs chord) and A0–A1 effort-in strip with frame playhead
- Spatial generate is a 3v3 (`generate_3v3_attack`) on a FIFA 105×68 m pitch (camera locked to bounds): G0 attacks +x (up, white), G1 defends (magenta). Summary lists team roll-ups and within/versus pairs
- `symworx-spatialsym::soccer` IFAB Law 1 presets (length 90–120 m, width 45–90 m; fixed 16.5/5.5 m boxes, 7.32 m goal, 11 m PK, 9.15 m circle). Generic `PlayAreaMarkings` drawn on the Spatial plan
- Spatial Visualize: stats on the left, field on the right (attack +x drawn upward). Phase edges stay cyan/yellow/red
- Spatial TUI summary shows path efficiency / RMS deviation and pairwise effort / directional phase
- Windowed pairwise phase (`*_series` / `*_at`) and signed closing acceleration (`a_close`); TUI `Now` line uses a 1 s window
- `path_and_phase` example: straight vs weave, together / opposed / one-go-one-brake drills, plus 3v3 player/team print
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

[0.3.2]: https://github.com/symworx/symworx/releases/tag/v0.3.2
[0.3.0]: https://github.com/symworx/symworx/releases/tag/v0.3.0
[0.2.0]: https://github.com/symworx/symworx/releases/tag/v0.2.0
[0.1.1]: https://github.com/symworx/symworx/releases/tag/v0.1.1
[0.1.0]: https://github.com/symworx/symworx/releases/tag/v0.1.0
