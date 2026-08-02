# SymWorx-Math

Numerical primitives for the SymWorx ecosystem. This crate is deliberately
**lightweight** (no LAPACK / OpenBLAS) so it can be shared freely across
stats, signal, dynamics, and domain crates without huge compile times.

## Contents

| Module | Role |
|--------|------|
| `series` | Successive differences, rolling stats, sliding windows (canonical home for sequence ops) |
| `integration` | Trapezoidal integration, RK4 ODE step/integrate |
| `optimize` | Gradient descent, Armijo line search, finite-difference gradients |
| `oscillators` | Van der Pol and related demo plants |
| `distributions` / `special` / `random` | PDFs, Gamma/Beta, RNG helpers |

## Where this crate sits

Optimization primitives here support nonlinear regression in `symworx-stats`
and parameter fitting for dynamical models. Heavy linear algebra (SVD, PCA)
lives in `symworx-stats` behind the `linalg` feature — **not** here.

## Usage example

```rust
use symworx_math::optimize::{gradient_descent_fd, GradientDescentConfig};
use ndarray::array;

let f = |p: &ndarray::Array1<f64>| (p[0] - 3.0).powi(2);
let cfg = GradientDescentConfig {
    learning_rate: 0.1,
    max_iter: 200,
    ..Default::default()
};
let result = gradient_descent_fd(f, array![0.0], &cfg);
assert!((result.params[0] - 3.0).abs() < 1e-4);
```

```rust
use symworx_math::integrate::rk4_integrate;
use ndarray::Array1;

let f = |t: f64, y: &Array1<f64>| y.clone();
let (times, states) = rk4_integrate(f, (0.0, 1.0), Array1::from(vec![1.0]), 0.1);
```
