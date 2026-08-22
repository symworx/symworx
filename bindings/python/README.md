# SymWorx Python bindings

Unified package **`symworx`**: PyO3 wrappers over the Rust crates (`biosym`, `loadsym`, `core`).

## Layout

| Path | Role |
|------|------|
| `src/` | Rust extension (`symworx._lib`) — PyO3 classes and functions |
| `symworx/` | Pure-Python package that re-exports from `_lib` |
| `tests/` | `pytest` against the installed package |
| `examples/` | Small demos |
| `pyproject-*.toml` | Optional **split** packages (`symworx_biosym`, …); secondary to the unified build |

Users import the public surface:

```python
from symworx import biosym, loadsym, core
```

The private extension is `symworx._lib` (attribute access from package code only).

## Develop & test

From the **workspace root**:

```bash
python -m venv .venv && .venv/bin/pip install "maturin>=1.5,<2.0" pytest
VIRTUAL_ENV=$PWD/.venv .venv/bin/maturin develop --manifest-path bindings/python/Cargo.toml
.venv/bin/pytest bindings/python/tests/ -q --tb=line
```

Optional IMAP helpers need OpenSSL and:

```bash
maturin develop --manifest-path bindings/python/Cargo.toml --features email
```

## Statistics / classical ML (`symworx.core.statistics`)

Rust `symworx-stats` surface exposed for 0.1 (in addition to basic mean/corr/l1/l2/HRV):

| API | Role |
|-----|------|
| `train_test_split` | Index-only train/test (+ optional folds) |
| `standard_scaler_fit` / `StandardScaler` | Fit/transform (train-only fit) |
| `logistic_regression` / `LogisticModel` | Binary logistic |
| `logistic_regression_ovr` / `MulticlassLogisticModel` | One-vs-rest multiclass |
| `gaussian_nb` / `GaussianNb` | Gaussian Naive Bayes |
| `lmer` / `MixedModel` | Linear mixed model (random intercept or linear growth) |
| `simulate_random_intercept` | Balanced LMM sim for tests / demos |
| `accuracy`, `classification_report` | Classification metrics |
| `roc_auc`, `roc_auc_ovr` | ROC-AUC binary / macro OVR |

```python
from symworx.core import statistics as st

plan = st.train_test_split(len(X), test_ratio=0.3, seed=42)
scaler, Xs = st.standard_scaler_fit([X[i] for i in plan.train_idx])
model = st.logistic_regression_ovr(Xs, [y[i] for i in plan.train_idx], max_iter=5000)
print(model.predict(scaler.transform([X[i] for i in plan.test_idx])))
```

Example: `python bindings/python/examples/stats_logistic.py`  
Tests: `pytest bindings/python/tests/test_core_statistics.py -q`

**Not yet bound:** rule lists, k-NN, LDA, polyreg, full ridge/ols objects (use `l1`/`l2` packed coeffs). See Rust examples for those.

**Note:** `l2` / full `symworx-core` builds pull OpenBLAS via stats `linalg`.