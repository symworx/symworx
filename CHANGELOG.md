# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **symworx-stats:** train/test splits (+ folds, repeated splits); binary and multiclass (OVR) logistic regression; Gaussian NB; LDA; k-NN; threshold rule lists + decision stump; classification metrics and ROC/AUC; StandardScaler / MinMaxScaler; polynomial degree search (with sample-size guards and optional residuals).
- **symworx-stats:** model export guide for embedded C, iOS, Android, and web (`docs/model_export.md`).
- **Python bindings:** `symworx.core.statistics` exposes train/test split, StandardScaler, binary + OVR logistic, Gaussian NB, classification_report, roc_auc / roc_auc_ovr (+ tests and `examples/stats_logistic.py`).
- **symworx-embed:** host-side PPG JSON protocol, simulator/serial sources, ring buffers (`sid` subject naming).
- Root README rewritten: persona quickstarts, supported vs experimental matrix, clean structure.

### Changed
- Polyreg warnings default to result-only (`print_warnings` opt-in); no library noise by default.

### Notes
- Python bindings still expose a **subset** of Rust stats (expand before tagging 0.1).
- Welch PSD remains a placeholder; do not rely on it for analysis.
- Workspace remains a monorepo with a shared version and single changelog.

## [0.1.0] - TBD

### Added
- Initial public-oriented layout of the SymWorx workspace (math, stats, signal, dynamics, biosym, loadsym, io, spatialsym, tui, Python bindings).
- Core primitives and domain crates as described in crate READMEs.

### Notes
- Target first tagged release after onboarding cleanup, stub honesty, and Python stats subset are complete (see release readiness notes).
- Branching model: `develop` / `staging` / `main` with squash merges for releases.

---

## Version Links

[Unreleased]: https://github.com/symworx/symworx/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/symworx/symworx/releases/tag/v0.1.0
