# SymWorx-Math

Numerical primitives for the SymWorx ecosystem (integration, special functions, distributions).

## Current Contents

- Trapezoidal integration (`trapz`, `cumtrapz`)
- **RK4 ODE integration** — `rk4_step` and `rk4_integrate` (used by BioSym CPG)
- Special functions (Gamma, Beta, ln versions)
- Basic distributions and kernels

This crate is designed to stay lightweight and reusable across SymWorx crates and beyond.

## Usage Example

```rust
use symworx_math::integrate::{rk4_step, rk4_integrate};
use ndarray::Array1;

let f = |t: f64, y: &Array1<f64>| y.clone(); // simple exponential
let (times, states) = rk4_integrate(f, (0.0, 1.0), Array1::from(vec![1.0]), 0.1);
```
