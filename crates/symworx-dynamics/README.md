# SymWorx-Dynamics

Nonlinear dynamics and data-driven dynamical systems tools for SymWorx.
This is a sub-crate to the [`symworx-core`](../symworx-core/README.md) crate.

## Modules

| Module | Methods |
|:-------|:--------|
| `embedding` | Delay coordinates (`edim`), false nearest neighbors (`fnn`) |
| `entropy` | Sample entropy, multiscale entropy, histogram AMI / cross-AMI, discrete transfer entropy |
| `rqa` | Recurrence plots, RQA / cRQA metrics |
| `phase` | Relative phase, Kuramoto *R*, cluster-phase (Richardson et al. 2012); input is pre-extracted phases |
| `dmd` | Dynamic Mode Decomposition (exact / SVD-based) |
| `koopman` | EDMD with identity / polynomial dictionaries |
| `sindy` | Sparse identification of nonlinear dynamics (STLS) |
| `sindyc` | SINDYc — SINDy with control (`Θ(x,u)`) |
| `control` | Discrete LTI plants, state feedback, PID |

## Transfer entropy

Discrete (histogram) Schreiber TE. Entropy in nats. Not a kNN / Kraskov estimator.

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

- `transfer_entropy_discrete*` — same estimator on already-discrete series (`u8` labels). `TeConfig::bins` is ignored. Use this for sleep stages or for HRV after `symworx_stats::RelativeKMeansDiscretizer`.

Returns `0.0` for short, constant, or mismatched series. Do **not** concatenate users or disconnected nights into one TE series — compute per contiguous record, then summarize.

Shared relative cuts (per-user min/max, k-means on the pooled unit interval) live in `symworx-stats::discretize`. Example: `cargo run -p symworx-dynamics --example relative_te_demo`.

No TUI binding. Python: `symworx.core.dynamics.transfer_entropy`, `transfer_entropy_with`, `transfer_entropy_mv`, `transfer_entropy_conditional`, and `transfer_entropy_discrete*`.

## Average mutual information

Histogram mutual information in nats. Bins are equal-occupancy quantiles, the same partition family as discrete transfer entropy. Default `bins` is 8 (`DEFAULT_AMI_BINS`).

```rust
use symworx_dynamics::{ami, cross_ami, mutual_information, first_local_minimum, DEFAULT_AMI_BINS};

let mi = mutual_information(&x, &y, DEFAULT_AMI_BINS);
let auto = ami(&x, 6, DEFAULT_AMI_BINS);
let tau = first_local_minimum(&auto);
let lagged = cross_ami(&driver, &target, 6, DEFAULT_AMI_BINS);
```

`cross_ami` index `d` is `I(driver[t−d]; target[t])`, the same backward shift as a Kalman control lag. Those bins are fit once on each full series. `mutual_information` refits bins on the pairs it is given: build the complete pairs yourself and do not concatenate visits. Lag 0 of `ami` is the entropy of `x` under that partition. The embedding readout is the first local minimum.

Short, constant, non-finite, or mismatched inputs return `0.0` or an empty curve. `bins` must be in `2..=255`.

Python: `symworx.core.dynamics.ami`, `cross_ami`, `mutual_information`, `mutual_information_discrete`, `first_local_minimum`. `symworx.core.statistics` exposes `acf_normalized` and `ccf` with the same lag sign.

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


- `transfer_entropy_discrete*` — same estimator on already-discrete series (`u8` labels). `TeConfig::bins` is ignored. Use this for sleep stages or for HRV after `symworx_stats::RelativeKMeansDiscretizer`.

Returns `0.0` for short, constant, or mismatched series. Do **not** concatenate users or disconnected nights into one TE series — compute per contiguous record, then summarize.

Shared relative cuts (per-user min/max, k-means on the pooled unit interval) live in `symworx-stats::discretize`. Example: `cargo run -p symworx-dynamics --example relative_te_demo`.

No TUI or Python binding yet.

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

## Citation

If you use this software, please cite it:

Berry, N. T. (2026). *SymWorx* [Computer software]. https://github.com/symworx/symworx

```bibtex
@software{Berry_SymWorx_2026,
  author  = {Berry, Nathaniel T.},
  license = {Apache-2.0},
  title   = {{SymWorx}},
  url     = {https://github.com/symworx/symworx},
  year    = {2026}
}
```

Add the version you used. [`CITATION.cff`](https://github.com/symworx/symworx/blob/main/CITATION.cff) and GitHub **Cite this repository** carry the current release.
