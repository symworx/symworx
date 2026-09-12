# SymWorx-Signal

Digital signal processing for physiological and biomechanical data: filters,
peaks, resampling, sparse sensing, and state estimation.

This is a sub-crate of [`symworx-core`](../symworx-core/README.md).

## Highlights

| Area | Modules |
|------|---------|
| Linear / adaptive filters | FIR/IIR, LMS/NLMS, Savitzky–Golay, RLS |
| Time-frequency | STFT, Hilbert, CWT, **Welch PSD** (`welch` / `welch_default`) |
| State estimation | `KalmanFilter` (+ RTS), `KalmanFilter1D`, **EKF**, **UKF** |
| Sparse sensing | ISTA, OMP, Gaussian Φ, DCT basis (`processing::sparse_sensing`) |
| Processing | peaks, resample (linear / cubic spline), outliers, windows, deconvolution (NNLS/Wiener) |

## Examples

```bash
# RR cleaning → tachogram → window features
cargo run -p symworx-signal --example windowed_rr_features

# Compressed sensing (OMP / ISTA) + regression_report scoring
cargo run -p symworx-signal --example sparse_sensing_demo

# Constant-velocity Kalman, EKF/UKF on sin(θ), control-input LTI
cargo run -p symworx-signal --example state_estimation_demo
```

## Quick Kalman constructors

```rust
use symworx_signal::filters::KalmanFilter;

let mut kf = KalmanFilter::constant_velocity_1d(1.0, 1e-4, 0.1);
kf.predict(None);
kf.update(&ndarray::array![1.2]);
```

Nonlinear measurement models:

```rust
use symworx_signal::filters::{ExtendedKalmanFilter, UnscentedKalmanFilter};
```

## Related

- **`symworx-stats`** — `regression_report`, clustering, regression (`--features linalg`)
- **`symworx-dynamics`** — DMD, EDMD, SINDy/SINDYc, LTI/PID

## Citation

If you use this software, please cite **SymWorx**, not this crate alone.
Name the crate in the paper if it helps; the bibliography title is still *SymWorx*.

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
