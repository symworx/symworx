# SymWorx-Dynamics

Nonlinear dynamics and data-driven dynamical systems tools for SymWorx.
This is a sub-crate to the [`symworx-core`](../symworx-core/README.md) crate.

## Modules

| Module | Methods |
|--------|---------|
| `embedding` | Delay coordinates (`edim`), false nearest neighbors (`fnn`) |
| `entropy` | Sample entropy, multiscale entropy |
| `rqa` | Recurrence plots, RQA / cRQA metrics |
| `dmd` | Dynamic Mode Decomposition (exact / SVD-based) |
| `koopman` | EDMD with identity / polynomial dictionaries |
| `sindy` | Sparse identification of nonlinear dynamics (STLS) |
| `sindyc` | SINDYc — SINDy with control (`Θ(x,u)`) |
| `control` | Discrete LTI plants, state feedback, PID |

## DMD / EDMD

```rust
use symworx_dynamics::{dmd, edmd, DmdConfig, EdmdConfig, Dictionary};

let dmd_model = dmd(&snapshots, &DmdConfig { rank: Some(4), ..Default::default() });
let edmd_model = edmd(&snapshots, &EdmdConfig {
    dictionary: Dictionary::Polynomial { max_degree: 2, include_constant: true },
    ridge: 1e-8,
});
```

## LTI + PID

```rust
use symworx_dynamics::{LtiDiscrete, Pid, PidConfig};
use ndarray::array;

let plant = LtiDiscrete::double_integrator(0.01);
let mut pid = Pid::gains(2.0, 0.1, 0.05, 0.01);
```

## Related

- **`symworx-stats`** — SVD, regression, clustering
- **`symworx-signal`** — Kalman / EKF / UKF, sparse sensing

### SINDy / SINDYc

```rust
use symworx_dynamics::{sindy, sindyc, SindyConfig, SindycConfig, Dictionary};

// Autonomous
let model = sindy(&snapshots, 0.01, &SindyConfig {
    dictionary: Dictionary::Polynomial { max_degree: 2, include_constant: true },
    threshold: 0.1,
    ..Default::default()
});

// With control: snapshots n×T, controls m×T
let forced = sindyc(&snapshots, &controls, 0.01, &SindycConfig::default());
let f = forced.rhs(&x0, &u0);
```
