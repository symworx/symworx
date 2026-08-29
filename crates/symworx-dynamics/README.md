# SymWorx-Dynamics

Nonlinear dynamics and data-driven dynamical systems tools for SymWorx.
This is a sub-crate to the [`symworx-core`](../symworx-core/README.md) crate.

## Modules

| Module | Methods |
|:-------|:--------|
| `embedding` | Delay coordinates (`edim`), false nearest neighbors (`fnn`) |
| `entropy` | Sample entropy, multiscale entropy, discrete transfer entropy |
| `rqa` | Recurrence plots, RQA / cRQA metrics |
| `phase` | Relative phase, Kuramoto *R*, cluster-phase (Richardson et al. 2012); input is pre-extracted phases |
| `dmd` | Dynamic Mode Decomposition (exact / SVD-based) |
| `koopman` | EDMD with identity / polynomial dictionaries |
| `sindy` | Sparse identification of nonlinear dynamics (STLS) |
| `sindyc` | SINDYc — SINDy with control (`Θ(x,u)`) |
| `control` | Discrete LTI plants, state feedback, PID |

## Transfer entropy

Discrete (quantile-binned) Schreiber TE. Entropy in nats. Not a kNN / Kraskov estimator.

```rust
use symworx_dynamics::{transfer_entropy, transfer_entropy_mv, transfer_entropy_conditional, TeConfig};

let te_xy = transfer_entropy(&x, &y);

let cfg = TeConfig { k: 1, l: 1, tau: 1, horizon: 1, bins: 4 };
let te_joint = transfer_entropy_mv(&[&x, &z], &y, &cfg);
let te_x_given_z = transfer_entropy_conditional(&[&z], &[&x], &y, &cfg);
```

- `transfer_entropy` / `transfer_entropy_with` — bivariate `X → Y`
- `transfer_entropy_mv` — joint sources `(X1,...,Xp) → Y`
- `transfer_entropy_conditional` — partial `X → Y | Z`

Returns `0.0` for short, constant, or mismatched series. No TUI or Python binding yet.

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

## Runnable example

```bash
# DMD + EDMD + SINDy + SINDYc + LTI/PID (scores DMD with regression_report)
cargo run -p symworx-dynamics --example data_driven_dynamics_demo
```

## Related

- **`symworx-stats`** — SVD, regression, clustering, `regression_report`
- **`symworx-signal`** — Kalman / EKF / UKF, sparse sensing

Design notes (not shipped APIs): [notes/group-phase-coherence.md](notes/group-phase-coherence.md)
— cluster-phase, MdRQA, and group-level CRQA summaries.

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
