---
title: 'SymWorx: A Rust-based framework for biomechanical and physiological simulation, signal processing, and nonlinear dynamics analysis'
tags:
  - rust
  - python
  - biomechanics
  - physiology
  - signal-processing
  - nonlinear-dynamics
  - central-pattern-generator
  - rqa
authors:
  - name: Nathaniel T. Berry
    affiliation: "1,2"
affiliations:
  - index: 1
    name: "University of North Carolina at Greensboro, Department of Information, Library, and Research Sciences"
  - index: 2
    name: "University of North Carolina at Greensboro, Department of Kinesiology; Computational Systems for Modeling and Dynamics (cSYMd) Laboratory"
date: DD-MM-2026
---

**Summary**

SymWorx is a modular Rust framework for simulating, processing, and analyzing biomechanical and physiological signals, with particular strength in nonlinear dynamical analysis via recurrence quantification analysis (RQA). 
The Rust core emphasizes performance, memory safety, and reproducibility. 
Key capabilities include central pattern generator (CPG) models based on coupled oscillators, photoplethysmography (PPG) and respiratory signal generation and analysis, gait modeling and spatiotemporal metrics, a broad suite of signal filters and processing routines, training load and nutrition optimization models, spatial/trajectory analysis, and a full implementation of recurrence quantification analysis (including recurrence plot construction and standard RQA metrics). 
An interactive terminal user interface (`symview`) provides tabbed workflows for importing real signals (CSV, Parquet, IBI), generating demo data directly from the biosignal simulators, exploring statistics and visualizations, and performing RQA — enabling transparent, code-optional exploration suitable for research and teaching.

**Statement of need**

Computational modeling and nonlinear time-series analysis are central to modern biomechanical and physiological research. 
Researchers frequently need to couple neural control models (e.g., central pattern generators), physiologically plausible signal generators, real-world data import, signal conditioning, and nonlinear measures such as sample entropy or recurrence quantification within reproducible pipelines. 
Many existing tools excel in one area — OpenSim for detailed musculoskeletal simulation, or the SciPy/NumPy ecosystem for general signal processing and statistics — but stitching these together with custom RQA or CPG implementations often produces fragile, heavyweight dependency graphs that are difficult to deploy consistently across platforms or to use in teaching settings where step-by-step transparency matters.

SymWorx addresses this by providing a single, auditable Rust foundation that implements the necessary generators, processors, and nonlinear tools natively, exposed both as libraries and through an integrated interactive terminal application. 
The design supports researchers who want full control over models and parameters as well as instructors and students who benefit from immediate, keyboard-driven exploration of simulated and real signals leading directly into RQA.

**State of the field**

Tools such as OpenSim provide mature musculoskeletal forward dynamics but limited native support for neural oscillators or recurrence-based nonlinear analysis. 
General-purpose scientific Python stacks are extremely capable yet encourage accumulation of complex transitive dependencies when building full analysis pipelines that include custom dynamical systems work. 
Standalone RQA implementations exist in several languages, but they are rarely packaged alongside matched physiological generators and an interactive environment that lets users move fluidly from "generate or load a signal" to "compute and visualize its recurrence plot."

SymWorx takes a pragmatic approach to dependencies: core array and FFT work rests on a small, well-audited set (ndarray, rustfft), while heavier linear-algebra functionality (via ndarray-linalg) is feature-gated so that common use cases and the Python bindings do not force native LAPACK/OpenBLAS requirements on every consumer. 
Most domain logic — CPG integration, gait metrics, PPG/respiration peak and interval analysis, the full RQA metric suite, and a wide range of linear/adaptive/time-frequency filters — is implemented directly in Rust.

**Software design**

SymWorx is organized as a Cargo workspace of focused crates behind a convenience `symworx-core` re-export layer. 
This structure keeps domain concerns (biosignal simulation and analysis, training load/nutrition, nonlinear dynamics, spatial analysis) separable while ensuring a consistent numerical and error foundation.

- `symworx-core` aggregates and re-exports common functionality (math primitives and series operations, signal processing, statistics, dynamics/RQA, I/O traits, error types) for ergonomic use across the workspace.
- `symworx-biosym` supplies physiological and biomechanical generators and analyzers: PPG and respiration waveform generation with tunable noise and quality presets; peak detection, interval series, basic HRV, and phase-aware respiration metrics; gait parameter and spatiotemporal metric models (stride/step length, symmetry, cadence, etc.); and a coupled-oscillator central pattern generator (`SymCpgModel`) using Van der Pol oscillators for cardio-locomotor-respiratory dynamics, integrated with fixed-step RK4.
- `symworx-dynamics` provides embedding (edim, false nearest neighbors), entropy measures (including sample entropy), and a complete recurrence quantification analysis (RQA) implementation, including `RecurrencePlot` construction and the standard suite of RQA metrics (recurrence rate, determinism, laminarity, Lmax, Lmean, diagonal line entropy/Lentr, trapping time, Vmax, etc.). Cross-recurrence (CRQA) support is planned.
- `symworx-signal` implements a broad collection of filters (FIR, IIR variants such as Bessel/Chebyshev, Savitzky–Golay, LMS/RLS adaptive, Kalman, STFT, wavelet, EMD, Hilbert) and processing utilities (peak finding, resampling, interpolation, normalization, outlier handling, deconvolution pipelines including NNLS/Weiner).
- `symworx-math` supplies low-level numerical primitives (series operations including successive differences and rolling stats, integration/RK4, random, special functions, oscillators including Van der Pol) — the canonical home for such routines.
- `symworx-stats` offers descriptive statistics, variability, correlation, autocorrelation, spectral methods, and regression/PCA/SVD (with an opt-in `linalg` feature for heavier linear algebra).
- `symworx-io` is the **single source of truth** for loading and saving real signals (CSV with headers, Parquet, IBI/RR intervals, activity/.fit files) and related traits. All other crates use this layer.
- `symworx-loadsym` (with `symworx-loadsym-db`) provides literature-grounded nutrition (energy expenditure, BMR, weight-loss modeling) and training-load calculations (ACWR, monotony/strain, optimization routines), plus the `symload` CLI.
- `symworx-spatialsym` provides sport-agnostic 2D trajectory and space analysis (kinematics, expansion/pressure/denial metrics, agent decision modeling) with synthetic data generation and post-hoc interpretation tools. Fully integrated into the TUI.
- `symworx-tui` (`symview`) is the primary interactive application. It features a Home workflow selector (launched by default or via `Ctrl+H`) offering BioSym (signal generation/analysis + RQA), LoadSym (training load/nutrition), and SpatialSym (trajectory/space analysis) paths, with dedicated tabs and sub-views for each. Designed for rapid, keyboard-centric exploration (see `crates/symworx-tui` and notes.md for details).
- Supporting crates include `symworx-backend` (process/server utilities), `symworx-error`, and `symworx-embed`.

Python bindings (PyO3 via maturin) are provided for the major crates and are usable either via the unified `symworx` namespace or individual subpackages (e.g., `symworx_biosym`). 
R bindings exist in early/stub form. 
The same Rust implementations are the source of truth for all language front-ends.

The architecture favors direct, testable implementations of domain algorithms in Rust so that results are reproducible across native binaries, the TUI, Python sessions, and (where the dependency graph permits) more constrained environments.

**Research impact**

SymWorx is designed to lower the barrier to integrated neural–biomechanical–physiological modeling and nonlinear time-series analysis while preserving transparency and auditability. 
The interactive `symview` TUI lets researchers and students generate controlled demo signals (e.g., resting PPG or respiration), load real recordings, inspect basic statistics and waveforms, and then directly compute and visualize recurrence plots and RQA metrics — all without leaving the terminal or assembling a large Python environment. 
This supports both rapid exploratory work and pedagogical use cases where the goal is to understand how parameter choices in a CPG or filter affect downstream nonlinear descriptors.

The software is used in the author's ongoing research in informatics, biomechanics, and physiology and is intended to support university-level teaching with transparent, step-by-step workflows. 
By keeping the core algorithms in a single, permissively licensed Rust codebase (Apache-2.0) with Python accessibility, SymWorx aims to facilitate reproducible research and reproducible teaching materials in these fields.

**AI Usage**

Generative AI tools were used to assist with drafting portions of this manuscript and suggesting improvements to documentation. 
All AI-generated content was reviewed, tested where applicable, and substantially edited by the author. 
No AI was used to generate the core scientific code in the SymWorx repository.

**Acknowledgements**

The author thanks ... 
. 
Development of SymWorx is supported in part by ... 
.

**References**
