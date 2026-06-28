# BioSym

**BioSym** is the biological systems modeling crate in the **SymWorx** ecosystem.
It provides tools for simulating and analyzing physiological and biomechanical signals, with a focus on gait, central pattern generators (CPG), and integrated cardio-locomotor-respiratory dynamics.

## Features (Current)

- **Gait modeling & analysis** — `GaitParams`, `GaitData`, `GaitStats`, `GaitAnalysis` (under `biomechanics::gait`) with stride detection from signals (`detect_gait_strides*`, `analyze_gait*`), quality presets, cadence, lengths, symmetry, vertical oscillation. Parity with physiology analysis.
- **Central Pattern Generator (CPG)** — Coupled Van der Pol oscillators for heart, bilateral legs, and respiration, driven by a dynamic `tau` parameter.
- **Numerical integration** — Uses RK4 from `symworx-math` for stable simulation.
- **Python bindings** — Full PyO3 support. Can be used standalone (`import symworx_biosym`) or via the unified `symworx` package.
- **Independent builds** — `maturin develop` works directly from the crate directory.

## Physiology Analysis

The `physiology` module provides generation + analysis for PPG and respiration (flow), built on shared primitives:

- **Common** (`physiology::common`): `PhysiologySignal`, `PhysiologySummary` (mean/std/dur), `IntervalSeries` (peaks, intervals, rates; supports alternating-phase split for legacy-style insp/exp), `HrvMetrics` (SDNN + RMSSD), `PhysiologyProcessingParams` (bandpass via `symworx-signal` biquads + peak overrides), peak detection via `symworx_core::PeakFinderBuilder`.
- **PPG**: `PpgAnalysis` (summary + intervals + mean HR bpm + HRV). `analyze_ppg*` / `detect_ppg_peaks*` / `summarize_ppg`. Quality presets (`PPGSignalQuality`: Reference/High/Moderate/Poor) drive bandpass (0.5–5 Hz) + tuned peak thresholds for noisy simulated data. Hardcoded default fs 250 Hz for signal wrapper.
- **Respiration**: `RespAnalysis` (summary + intervals + mean BRPM + insp/exp splits from alt phases + `RespPhasePeaks` from signed flow local maxima + phase-specific intervals). `analyze_respiration*` etc. Bandpass 0.1–0.5 Hz; default fs 50 Hz on flow channel. Volume field present but analysis focuses on flow.
- **Bindings**: Full `PpgAnalysis` / `RespAnalysis` (flattened for py) + analyze fns exposed.

See `physiology::{ppg,respiration}::analysis` and tests for details. Heavily reuses core crates; no direct scipy equivalent.

**Known gaps** (advanced / future):
- Waveform morphology (PPG: rise time/notch/augmentation; resp: I:E, peak flows, volume integrals).
- Extended HRV (pNN50, freq-domain LF/HF, nonlinear — use `symworx-dynamics` entropy + `symworx-stats` spectral for now).
- Cardiorespiratory coupling / RSA metrics (CPG has couplings; dedicated cross-analysis pending).
- Sleep module (legacy only).
- Real-sensor vs sim-tuned quality presets.
- Streaming / incremental analysis.

These are out of scope for current TUI/Dynamics focus (clean HR/BR + intervals + basic variability for RQA). See plan session for full evaluation.

## Biomechanics Analysis (Status)

(See implementation plan in session notes for current scoping of gait event detection, `GaitStats`/`GaitAnalysis`, quality presets, CPG integration, and bindings completion. Current surface is the `GaitData` calculators + pure metrics in `biomechanics::gait`.)

## Philosophy

SymWorx emphasizes security, robustness, and scalability. BioSym follows the same principles with strong typing, minimal unsafe code, and clean APIs suitable for embedded systems, research, and education.

## Usage (Rust)

```rust
use symworx_biosym::{GaitParams, GaitData, SymCpgModel};

let params = GaitParams::default().with_defaults();
let mut data = GaitData::new(100.0);
data.stride_times = Some(ndarray::array![0.0, 1.0, 2.0]);
data.calculate_stride_intervals();

let model = SymCpgModel::new(None);
let (times, states) = model.run((0.0, 10.0), 0.01);
```

## Usage (Python)

```python
import symworx_biosym as biosym

params = biosym.GaitParams()
model = biosym.SymCpgModel()
times, states = model.run(0.0, 10.0, 0.01)
```

**License:** Apache-2.0
