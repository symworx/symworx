# BioSym

**BioSym** is the biological systems modeling crate in the **SymWorx** ecosystem.
It provides tools for simulating and analyzing physiological and biomechanical signals, with a focus on gait, central pattern generators (CPG), and integrated cardio-locomotor-respiratory dynamics.

## Features (Current)

- **Gait modeling** — `GaitParams` and `GaitData` with stride interval, cadence, stride/step length, and vertical oscillation calculations.
- **Central Pattern Generator (CPG)** — Coupled Van der Pol oscillators for heart, bilateral legs, and respiration, driven by a dynamic `tau` parameter.
- **Numerical integration** — Uses RK4 from `symworx-math` for stable simulation.
- **Python bindings** — Full PyO3 support. Can be used standalone (`import symworx_biosym`) or via the unified `symworx` package.
- **Independent builds** — `maturin develop` works directly from the crate directory.

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

**License:** MPL-2.0
