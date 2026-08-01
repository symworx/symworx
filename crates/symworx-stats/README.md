# SymWorx-Stats

Statistical analysis and classical ML tools for the SymWorx ecosystem
(physiological signals, biomechanics, training load, and general scientific data).
This is a sub-crate of [`symworx-core`](../symworx-core/README.md).

## Module map

Statistics and machine-learning algorithms. Dynamical operators and signal sensing live elsewhere.

| Module | Methods | `linalg` feature? |
|:-------|:--------|:------------------|
| `basic`, `variability` | mean, median, MAD, RMSSD, … | No |
| `correlation`, `autocorrelation` | Pearson, ACF | No |
| `distance` | Euclidean, Manhattan, cosine, Chebyshev | No |
| `error_metrics` | MAE/MSE/RMSE, R², bias, residuals, `RegressionReport` | No |
| `classification_metrics` | Accuracy, confusion, F1, balanced acc, **ROC/AUC** (+ OVR) | No |
| `preprocess` | `StandardScaler`, `MinMaxScaler` (fit on train only) | No |
| `knn` | Multiclass k-NN (Euclidean/Manhattan/Cosine; vote proba) | No |
| `rules` | Threshold conditions, first-match rule lists, decision stump | No |
| `cluster` | k-means (+ k-means++), inertia, predict | No |
| `split` | Index-based train/test (+ folds / repeated resplits); min = max(10, 10% parent) | No |
| `linreg` | OLS, Ridge, Lasso, Elastic Net, soft-threshold | OLS/Ridge yes; Lasso/EN no |
| `logistic` | Binary + **multiclass OVR** logistic (GD; L2 optional) | No |
| `naive_bayes` | Gaussian Naive Bayes | No |
| `lda` | Linear Discriminant Analysis (linear scores for embed) | **Fit yes** |
| `polyreg` | Univariate polynomial degree sweep (hard/soft n rules) | **Yes** (OLS) |
| `nlinreg` | Nonlinear least squares (via `symworx-math::optimize`) | No |
| `svd` | SVD, rank-k truncate/reconstruct | **Yes** |
| `pca` | PCA fit/transform/whiten (uses SVD) | **Yes** |
| `spectral` | Welch PSD (stub → full implementation planned) | No |
| `synthetic` | Teaching presets (Normal1D, bivariate, linear, class blobs, clusters) | No |

### Feature flag

```toml
[dependencies]
symworx-stats = { path = "...", features = ["linalg"] }
```

`linalg` pulls `ndarray-linalg` + OpenBLAS. `symworx-core` enables it by default.

## Model export (embedded / mobile / web)

Train in Rust, ship coefficients or rules, run **predict-only** elsewhere:

→ **[docs/model_export.md](docs/model_export.md)** — C/MCU, iOS (Swift), Android (Kotlin), and TypeScript/web examples for scaler + logistic (binary/OVR), LDA, and rule lists.

## Related crates

- **`symworx-math`** — optimization / integration primitives (no LAPACK)
- **`symworx-signal`** — Kalman / EKF / UKF, sparse sensing
- **`symworx-dynamics`** — embedding, RQA, DMD, EDMD, SINDy/SINDYc, LTI/PID

## Runnable examples

```bash
# Simple fit/predict (no splits)
cargo run -p symworx-stats --example linear_regression_demo --features linalg
cargo run -p symworx-stats --example logistic_regression_demo
cargo run -p symworx-stats --example multiclass_logistic_demo
cargo run -p symworx-stats --example rule_list_demo
cargo run -p symworx-stats --example classification_metrics_demo
cargo run -p symworx-stats --example gaussian_nb_demo
cargo run -p symworx-stats --example knn_demo
cargo run -p symworx-stats --example roc_auc_demo
cargo run -p symworx-stats --example lda_demo --features linalg
cargo run -p symworx-stats --example polynomial_degree_search_demo --features linalg

# ML pipeline: train/test split + k-fold CV
cargo run -p symworx-stats --example linear_ml_pipeline_demo --features linalg
cargo run -p symworx-stats --example logistic_ml_pipeline_demo

# Index-based splits only
cargo run -p symworx-stats --example train_test_split_demo

# Broader suite: OLS family, k-means, PCA, regression_report
cargo run -p symworx-stats --example predictive_metrics_demo --features linalg
```

## Quick snippets

### Predicted vs expected

```rust
use symworx_stats::regression_report;

let y = [1.0, 2.0, 3.0, 4.0];
let yhat = [1.1, 1.9, 3.2, 3.8];
let rep = regression_report(&y, &yhat);
// residual convention: e = y − ŷ  (positive bias ⇒ under-prediction)
println!("{rep}"); // n=… MAE=… RMSE=… R²=… bias=… max|e|=…
```

### Clustering

```rust
use symworx_stats::{kmeans, KMeansConfig};
use ndarray::array;

let data = array![[0.0, 0.0], [0.1, 0.0], [5.0, 5.0], [5.1, 5.0]];
let result = kmeans(&data, &KMeansConfig { k: 2, ..Default::default() });
```

### Logistic regression (binary)

```rust
use symworx_stats::{logistic_regression, LogisticConfig};
use ndarray::array;

let x = array![[0.0], [0.2], [0.8], [1.0]];
let y = array![0.0, 0.0, 1.0, 1.0];
let model = logistic_regression(&x, &y, &LogisticConfig::default());
let p = model.predict_proba(&x); // P(y = 1)
let yhat = model.predict(&x, 0.5);
```

### Rule list (thresholds)

```rust
use symworx_stats::{
    RuleListClassifier, ClassificationRule, Comparison, ThresholdCondition,
};

let clf = RuleListClassifier::new(
    vec![
        ClassificationRule {
            conditions: vec![
                ThresholdCondition::new(0, Comparison::Ge, 160.0),
                ThresholdCondition::new(1, Comparison::Lt, 20.0),
            ],
            label: 1,
            name: Some("high_load".into()),
        },
    ],
    0, // default
    [0, 1],
);
let yhat = clf.predict(&x);
```

### Multiclass logistic (one-vs-rest)

```rust
use symworx_stats::{logistic_regression_ovr, LogisticConfig};
use ndarray::array;

let x = array![[0.0, 0.0], [0.1, 0.0], [5.0, 5.0], [0.0, 5.0]];
let y = vec![0, 0, 1, 2];
let model = logistic_regression_ovr(&x, &y, &LogisticConfig::default());
let pred = model.predict(&x);           // class labels
let proba = model.predict_proba(&x);    // n × K, rows sum to 1
```

### Train / test split (indices only)

```rust
use symworx_stats::{train_test_split, take_indices_cloned, SplitConfig};

// Need enough train rows for folds: 10-fold needs n_train ≥ 100 (fold ≥ 10).
let rows: Vec<f64> = (0..200).map(|i| i as f64).collect();
let plan = train_test_split(
    rows.len(),
    &SplitConfig {
        test_ratio: 0.3,
        n_train_folds: Some(10),
        shuffle: true,
        seed: 42,
    },
)
.expect("valid split");
// Original data untouched — subset when needed:
let train = take_indices_cloned(&rows, &plan.train_idx);
let test = take_indices_cloned(&rows, &plan.test_idx);
```

```rust
// Requires features = ["linalg"]
use symworx_stats::{ols, ridge, Pca, Svd};
```
