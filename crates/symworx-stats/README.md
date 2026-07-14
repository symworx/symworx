# SymWorx-Stats

Statistical analysis and classical data-driven modeling tools for the SymWorx
ecosystem (physiological signals, biomechanics, training load, and general
scientific data).

## Module map (DDSE-aligned)

Methods from *Data-Driven Science and Engineering* (Brunton & Kutz) and related
classical ML live here unless they are dynamical operators or signal sensing.

| Module | Methods | `linalg` feature? |
|--------|---------|-------------------|
| `basic`, `variability` | mean, median, MAD, RMSSD, … | No |
| `correlation`, `autocorrelation` | Pearson, ACF | No |
| `distance` | Euclidean, Manhattan, cosine, Chebyshev | No |
| `error_metrics` | MAE/MSE/RMSE, R², bias, residuals, `RegressionReport` | No |
| `cluster` | k-means (+ k-means++), inertia, predict | No |
| `linreg` | OLS, Ridge, Lasso, Elastic Net, soft-threshold | OLS/Ridge yes; Lasso/EN no |
| `nlinreg` | Nonlinear least squares (via `symworx-math::optimize`) | No |
| `svd` | SVD, rank-k truncate/reconstruct | **Yes** |
| `pca` | PCA fit/transform/whiten (uses SVD) | **Yes** |
| `spectral` | Welch PSD (stub → full implementation planned) | No |

### Feature flag

```toml
[dependencies]
symworx-stats = { path = "...", features = ["linalg"] }
```

`linalg` pulls `ndarray-linalg` + OpenBLAS. `symworx-core` enables it by default.

## Related crates

- **`symworx-math`** — optimization / integration primitives (no LAPACK)
- **`symworx-signal`** — Kalman, filters, sparse sensing (`processing::sparse_sensing`)
- **`symworx-dynamics`** — embedding, RQA, DMD (`dmd`); Koopman/SINDy planned

## Quick examples

### Predicted vs expected

```rust
use symworx_stats::regression_report;

let y = [1.0, 2.0, 3.0, 4.0];
let yhat = [1.1, 1.9, 3.2, 3.8];
let rep = regression_report(&y, &yhat);
// residual convention: e = y − ŷ  (positive bias ⇒ under-prediction)
println!("{rep}"); // n=… MAE=… RMSE=… R²=… bias=… max|e|=…
assert!(rep.r2.is_finite());
```

### Clustering

```rust
use symworx_stats::{kmeans, KMeansConfig};
use ndarray::array;

let data = array![[0.0, 0.0], [0.1, 0.0], [5.0, 5.0], [5.1, 5.0]];
let result = kmeans(&data, &KMeansConfig { k: 2, ..Default::default() });
assert_eq!(result.labels.len(), 4);
```

```rust
// Requires features = ["linalg"]
use symworx_stats::{ols, ridge, Pca, Svd};
```
