# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-07-31

### Added
- Initial public-oriented layout of the SymWorx workspace (math, stats, signal, dynamics, biosym, loadsym, io, spatialsym, embed, tui, Python bindings).
- **symworx-stats:** train/test splits (+ folds, repeated splits); binary and multiclass (OVR) logistic regression; Gaussian NB; LDA; k-NN; threshold rule lists + decision stump; classification metrics and ROC/AUC; StandardScaler / MinMaxScaler; polynomial degree search (with sample-size guards and optional residuals).
- **symworx-stats:** model export guide for embedded C, iOS, Android, and web (`docs/model_export.md`).
- **Python bindings:** `symworx.core.statistics` exposes train/test split, StandardScaler, binary + OVR logistic, Gaussian NB, classification_report, roc_auc / roc_auc_ovr (+ tests and `examples/stats_logistic.py`).
- **symworx-embed:** host-side PPG JSON protocol, simulator/serial sources, ring buffers (`sid` subject naming).
- Root README rewritten: persona quickstarts, supported vs experimental matrix, clean structure.
- Release path: `develop` → `stage` → `release/vX.Y.Z` → `main` (+ tag); release validation workflow (crates.io publish remains manual/disabled in CI).

### Changed
- Polyreg warnings default to result-only (`print_warnings` opt-in); no library noise by default.
- Internal crate dependencies inherit path + version from `[workspace.dependencies]` for consistent monorepo builds and crates.io publishing.

### Notes
- Python bindings still expose a **subset** of Rust stats (expand in a later release).
- Welch PSD remains a placeholder; do not rely on it for analysis.
- Workspace remains a monorepo with a shared version and single changelog.
- Scientific manuscripts live in a separate publications repo; this monorepo keeps a `publications/` placeholder for future reference copies.

---

## Version Links

[Unreleased]: https://github.com/symworx/symworx/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/symworx/symworx/releases/tag/v0.1.0
