# JOSS 2026 Submission Notes

**Paper location**: `publications/joss/2026/paper.md`

**Date of notes**: 2026-07-06 (current dev state on nb/joss-2026 / develop)

This file tracks mismatches between the paper and the actual codebase, plus tasks to resolve before JOSS submission. Use it to track progress on features and paper updates.

## Critical Paper Updates Completed (in this session)

- License string updated from MPL-2.0 → Apache-2.0 (in Research impact section).
- TUI description expanded/updated in Software design to reflect current workflow + tab architecture.
- Crates and capabilities list expanded to full current set of workspace members.
- `paper.md` relocated here from repo root; this `notes.md` created for tracking.

## Known Mismatches / Functionality Gaps (Paper vs. Implementation)

### 1. TUI / symview Architecture and Scope
- **Paper (pre-update) claimed**: "presents three tabs — Import (Tab 1), Explore (Tab 2), and Dynamics (Tab 3)".
- **Reality (current)**:
  - `Workflow` selector on launch / via Ctrl+H (Home screen): BioSym, LoadSym (2), SpatialSym (3).
  - BioSym path tab bar: 1:Import, 2:Explore, 3:Dynamics, 4:Spatial.
  - LoadSym has rich internal views: List, Workout (with .fit/activity loading, ACWR/monotony/optimization), Calendar, Optimization.
  - Spatial has sub-views: Visualize, Generate, ImportData. Uses symworx-spatialsym for trajectories, decisions, space metrics.
  - Keybinds and input priority (filter mode, pending_generate, column picker, etc.) are sophisticated and documented in code/AGENTS.md.
- **Paper impact**: Summary, Software design, and Research impact sections were outdated.
- **Status**: Updated in current paper.md. Re-verify against `crates/symworx-tui/src/{app.rs,ui/tabs/*,input.rs}` on future TUI changes.
- **Action**: Keep paper description high-level but accurate. Mention "Home workflow selector" + the three primary domains.

### 2. Crates and Capabilities
- **Paper previously listed** (incomplete): biosym, dynamics, signal, math, stats, io, loadsym, tui (+ core + Python).
- **Full current workspace members** (from root Cargo.toml):
  - `symworx-backend` — process/server/shutdown management.
  - `symworx-biosym` — PPG, respiration, gait, CPG (SymCpgModel + VanDerPol), fatigue, intensity, run performance.
  - `symworx-core` — re-exports + common series ops, error, etc.
  - `symworx-dynamics` — embedding (edim/fnn), entropy (sample_entropy), RQA (full metrics + RecurrencePlot; CRQA planned).
  - `symworx-embed` — small embed binary/tool.
  - `symworx-error` — SymError.
  - `symworx-io` — canonical I/O (CSV, Parquet, IBI, Activity/.fit, traits). **All other crates must use this for signal files.**
  - `symworx-loadsym` + `symworx-loadsym-db` — nutrition (BMR/weightloss), training load (ACWR, monotony, strain, optimization), symload CLI, .fit + email support (partial).
  - `symworx-math` — series, integration (RK4), oscillators (VanDerPol), random, distributions, special.
  - `symworx-signal` — extensive filters (linear/FIR/IIR/bessel/cheby, adaptive/LMS/RLS/SG, nonlinear/Kalman/RLS, time-freq/STFT/wavelet/EMD/Hilbert), processing (peaks, resample, deconvolution/NNLS/Weiner, outliers, etc.).
  - `symworx-spatialsym` — 2D trajectories, kinematics, space metrics (expansion/pressure/denial), agent decisions, synthetic data. Full TUI integration.
  - `symworx-stats` — basic/variability/correlation/autocorr/spectral, linreg/PCA/SVD (linalg feature opt-in).
  - `symworx-tui` (symview) — see TUI section.
- **Bindings**: Python (maturin, unified `symworx` + per-domain like `symworx_biosym`, `symworx_loadsym`; also separate pyprojects). R bindings: early/stub only.
- **Status**: Paper updated with fuller list. May still need light pruning for JOSS length (JOSS papers are concise).
- **Action**: When adding crates or major features, update both paper "Software design" bullets and this notes file.

### 3. RQA / Nonlinear Dynamics
- Full RQA implemented and used in TUI Dynamics (metrics: RR/DET/LAM/Lmax/Lmean/Lentr/TT/Vmax + sample entropy; RecurrencePlot construction + downsampled unicode preview; param editor).
- `rqa`, `rqa_from_trajectory`, embedding helpers exposed.
- CRQA: Still only "planned" (rectangular matrix support in utils; no CRQA API yet). Code comments + AGENTS.md note this.
- **Status**: Paper correctly qualifies CRQA. Matches "complete ... RQA implementation".
- **Gaps to track**: No full interactive recurrence plot matrix viewer beyond tiny preview; export of full matrix is planned via 'e'.

### 4. Other Feature / Scope Mismatches
- **LoadSym in TUI**: Now first-class (not just library). Paper previously under-emphasized.
- **SpatialSym**: New major domain + TUI workflow. Not mentioned pre-update.
- **Demo generation**: Works (Ctrl+G in Import). Generated files have headers; signal column (index 1) is loaded. Good.
- **I/O rule**: Strictly followed (symworx-io for all real signal load/save). Paper mentions import formats correctly.
- **Python**: Examples + tests exist for gait/nutrition. Unified + subpackage imports work.
- **R bindings**: Listed as "early form" — still accurate (Cargo stub, no substantial extendr code).
- **"symview" binary**: Confirmed in symworx-tui/Cargo.toml.
- **Filters/processing**: Paper list is accurate and comprehensive.

### 5. Paper Metadata & Formatting Issues
- **License**: Fixed (Apache-2.0).
- **Affiliations YAML**: Broken (author only "1"; duplicate index: 2 for dept + cSYMd lab). Needs repair for correct JOSS rendering.
  ```yaml
  # Suggested fix direction (verify JOSS template):
  authors:
    - name: Nathaniel T. Berry
      affiliation: "1,2"
  affiliations:
    - index: 1
      name: "... Information, Library, and Research Sciences"
    - index: 2
      name: "... Kinesiology; Computational Systems for Modeling and Dynamics (cSYMd) Laboratory"
  ```
- `date: DD-MM-2026` — placeholder.
- **Acknowledgements** + **References**: Still empty/placeholders. Need real content before submission.
- **AI Usage** section: Present and good.
- Length/structure: Follows JOSS-ish template (Summary / Statement of need / State of the field / Software design / Research impact). May benefit from trimming or aligning exactly to latest JOSS paper.md template (https://joss.readthedocs.io).

### 6. Project-Level Gaps for JOSS Review Criteria
JOSS requires (high level):
- **License**: Now good (Apache-2.0, OSI-approved). Root + per-crate.
- **README**: Exists but contains some terminal-capture artifacts / outdated sections (e.g., old "A0: (0.0..." text). Needs cleanup.
- **Contributing / docs**: CONTRIBUTING.md present; DEVELOPMENT.md is **empty** (0 bytes); AGENTS.md is internal/agent guidance (fine).
- **Tests**: Unit tests exist in several crates (e.g., biosym CPG tests, loadsym, dynamics). Python tests present. `cargo test` and `cargo check --workspace` work. **No visible CI** (`.github/` empty; no workflows).
- **CI / automation**: **Major gap** — no GitHub Actions visible for build/test/lint across platforms. JOSS reviewers expect automated testing.
- **Examples**: Good per-crate examples + TUI example + Python examples.
- **Versioning / releases**: Currently all 0.1.0 (dev). Paper date is 2026. Consider a tagged release or clear "pre-release" note.
- **Documentation**: Crate-level READMEs + rustdoc (`#![warn(missing_docs)]` in core crates). TUI has in-app help. Python docs light.
- **Installation**: Rust (cargo), Python (maturin). Instructions scattered; improve root README + a dedicated install section.
- **Community / issue tracker**: Assumes GitHub (repo url in Cargo). No CODE_OF_CONDUCT visible in quick scan.
- **Paper claims audit**: Must match delivered code at submission time (no over-claiming unimplemented features).

**Progress (2026-07-06)**:
- Confirmed via background check: `.github/workflows` does not exist in source (only build artifacts from OpenBLAS). Dynamics tests exist but many are ignored / light in default runs. Python tests (`test_biosym_gait.py`, `test_loadsym_nutrition.py`) are present.
- Added starter `.github/workflows/ci.yml` with:
  - `rust-checks` job: `cargo check --workspace`, selective tests on lighter crates, clippy (key crates), `cargo fmt --check`.
  - Dedicated `tui-check` job (plain + `--features polars`).
  - `python-bindings` job using maturin + pytest.
- Note in the workflow: heavy `ndarray-linalg` + OpenBLAS builds are intentionally not run on every push (very slow in CI). They can be added later as a manual / nightly / release job.

**High-priority project tasks for JOSS** (add to this file as you address):
- [ ] Add `.github/workflows/` with CI (test, check, clippy, python maturin build at minimum). Matrix for Linux + others.
- [ ] Populate or remove DEVELOPMENT.md. Flesh out contribution guide.
- [ ] Clean README.md (remove garbage output, improve quickstart for `cargo run -p symworx-tui --bin symview`, python install, common examples).
- [ ] Fix paper frontmatter (affiliations, date, acks, refs).
- [ ] Ensure all major public APIs have docs + examples.
- [ ] Consider adding a CHANGELOG or release notes.
- [ ] Verify paper builds cleanly with JOSS tooling (they use `joss` or pandoc + review checklist).

## Next Steps / Paper Polish
- Re-read paper.md after each major crate/TUI change.
- Keep functionality claims conservative ("provides", "implements") and qualify planned items.
- When TUI or new domains evolve, sync the Software design + Summary + Research impact paragraphs.
- Target JOSS review: short paper (~1000-2000 words?), clear "why this software", evidence it solves a need, and that the software is usable/reproducible.

Track updates here. When ready for submission, the paper.md in this dir + supporting repo state will be reviewed together.

See also: root AGENTS.md (development conventions), CONTRIBUTING.md, crate READMEs.
