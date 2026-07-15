# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

<!-- Insert notes here -->


## [0.1.0] - 2026-07-XX

### Added
- NOTE: Future changelogs will contain crate-level sections for easier tracing and potential migrations. 
- Initial public release of the SymWorx workspace.
- Core mathematical and statistical primitives (`symworx-math`, `symworx-stats`).
- Signal processing library with filtering, peak detection, and feature extraction (`symworx-signal`).
- Nonlinear dynamics tools including RQA/CRQA, embedding, entropy, DMD, SINDy, and Koopman analysis (`symworx-dynamics`).
- Biosignal modeling and simulation (PPG, respiration, gait, central pattern generators) in `symworx-biosym`.
- Training load quantification and optimization tools (ACWR, monotony, strain, nutrition, FIT file support) in `symworx-loadsym`.
- Interactive terminal UI (`symview`) for biosignal exploration, dynamics analysis, and load metrics (`symworx-tui`).
- Python bindings via PyO3 for core, biosym, loadsym, and spatialsym functionality.
- Spatial trajectory and decision-making analysis (`symworx-spatialsym`).
- I/O layer supporting Parquet, CSV, FIT, and other physiological data formats (`symworx-io`).

### Changed
- Reorganized several modules during the transition to the unified workspace structure.
- Consolidated former standalone `runsym` functionality into `symworx-biosym`.

### Fixed
- Multiple bugs fixed along the way during development. These will be tracked in a more meaningful way in the future. 

### Notes
- This is the first public release (`v0.1.0`).
- THis is a monorepo and all crates share a single version -- and a single CHANGELOG.md file.
- The repository uses a `develop` / `staging` / `main` branching model with squash merges for releases.

---

## Version Links

[Unreleased]: https://github.com/symworx/symworx/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/symworx/symworx/releases/tag/v0.1.0
