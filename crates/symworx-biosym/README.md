# BioSym

**BioSym** is the biological systems modeling crate in the **SymWorx** ecosystem.
It provides tools for simulating and analyzing physiological and biomechanical signals, with a focus on gait, central pattern generators (CPG), and integrated cardio-locomotor-respiratory dynamics.

## Features (Current)

- **Gait modeling & analysis** — `GaitParams`, `GaitData`, `GaitStats`, `GaitAnalysis` (under `biomechanics::gait`) with stride detection from signals (`detect_gait_strides*`, `analyze_gait*`), quality presets, cadence, lengths, symmetry, vertical oscillation. Parity with physiology analysis.
- **Central Pattern Generator (CPG)** — Coupled Van der Pol oscillators for heart, bilateral legs, and respiration, driven by a dynamic `tau` parameter.
- **Numerical integration** — Uses RK4 from `symworx-math` for stable simulation.
- **Python bindings** — via the unified package (`from symworx import biosym`); optional split package `symworx_biosym` may still exist in `bindings/python/`.

## Physiological Analysis

The `physiology` module provides generation + analysis for PPG and respiration (flow), built on shared primitives:

- **Common** (`physiology::common`): `PhysiologySignal`, `PhysiologySummary` (mean/std/dur), `IntervalSeries` (peaks, intervals, rates; optional alternating-phase insp/exp split), `HrvMetrics` (SDNN + RMSSD), `PhysiologyProcessingParams` (bandpass via `symworx-signal` biquads + peak overrides), peak detection via `symworx_core::PeakFinderBuilder`.
- **PPG**: `PpgAnalysis` (summary + intervals + mean HR bpm + HRV). `analyze_ppg*` / `detect_ppg_peaks*` / `summarize_ppg`. Quality presets (`PPGSignalQuality`: Reference/High/Moderate/Poor) drive bandpass (0.5–5 Hz) + tuned peak thresholds for noisy simulated data. Hardcoded default fs 250 Hz for signal wrapper.
- **Respiration**: `RespAnalysis` (summary + intervals + mean BRPM + insp/exp splits from alt phases + `RespPhasePeaks` from signed flow local maxima + phase-specific intervals). `analyze_respiration*` etc. Bandpass 0.1–0.5 Hz; default fs 50 Hz on flow channel. Volume field present but analysis focuses on flow.
- **Bindings**: Full `PpgAnalysis` / `RespAnalysis` (flattened for py) + analyze fns exposed.

See `physiology::{ppg,respiration}::analysis` and tests for details. Heavily reuses core crates; no direct scipy equivalent.

**Known gaps** (advanced / future):
- Waveform morphology (PPG: rise time/notch/augmentation; resp: I:E, peak flows, volume integrals).
- Extended HRV analysis (pNN50, freq-domain LF/HF, nonlinear will use `symworx-dynamics` entropy + `symworx-stats` spectral for now).
- Cardiorespiratory coupling / RSA metrics (CPG has couplings; dedicated cross-analysis pending).
- Sleep module and data simulation.
- Real-sensor vs sim-tuned quality presets.
- Streaming / incremental analysis.

These advanced metrics are future work; current focus is clean HR/BR + intervals + basic variability for RQA-style analysis.

## Biomechanics analysis

Gait event detection, `GaitStats` / `GaitAnalysis`, quality presets, and calculators live under `biomechanics::gait`.

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
from symworx import biosym

params = biosym.GaitParams()
# CPG: biosym.SymCpgModel (when built via unified bindings)
```

**License:** Apache-2.0
