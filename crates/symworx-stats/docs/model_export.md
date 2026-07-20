# Model export for embedded, mobile, and web

This guide explains how to **train in SymWorx (Rust / workstation)** and **run inference** on other platforms. The design goal is:

> Fit offline → ship a small bundle of numbers (or rules) → predict with a few multiplies and compares.

You do **not** need OpenBLAS, `ndarray`, or the full `symworx-stats` crate on the target. Only the **predict** math.

---

## 1. What is “export”?

| Term | Meaning here |
|------|----------------|
| **Train** | Fit models in Rust (`logistic_regression`, `lda`, `RuleListClassifier`, …) |
| **Export** | Copy fitted parameters into constants / JSON / assets for another runtime |
| **Infer** | Apply the same formulas on device / app / browser |

Export is **not** (yet) automatic codegen. You print or serialize coefficients once, paste them into the target, and keep feature order/units identical.

### Models that export cleanly

| Model | Payload | On-device cost |
|-------|---------|----------------|
| `StandardScaler` | `mean[]`, `scale[]` | O(p) |
| Binary logistic | `intercept`, `beta[]` | O(p) + sigmoid |
| Multiclass OVR logistic | per-class `intercept`, `beta[]` | O(K·p) |
| LDA | per-class `coef[]`, `intercept` | O(K·p) |
| Rule list / stump | feature index, op, threshold, label | O(rules) compares |
| Gaussian NB | priors, means, vars | O(K·p) |

### Do **not** treat as portable “coeffs only”

| Model | Why |
|-------|-----|
| k-NN | Needs the full training table |
| Polynomial degree *search* | Export the **chosen** degree’s β (via `LinearModel`), not the search object |
| PCA / SVD | Export loadings separately if you project features first |

---

## 2. Training-side checklist (Rust)

1. **Freeze feature order** — e.g. `[hr_bpm, rmssd_ms, …]`. Same order everywhere.  
2. **Fit scaler on train only**, then transform train/val/test.  
3. **Fit the model** on scaled (or raw, if you choose) features.  
4. **Record**:
   - scaler `mean`, `scale`
   - model intercept(s) / coefficients / rules
   - class label mapping (`0 = normal`, `1 = high_load`, …)
   - whether inputs are scaled  
5. **Smoke-test** one known row in Rust and on the target; scores should match within float noise.

### Example: dump binary logistic + scaler

```rust
use ndarray::array;
use symworx_stats::{
    LogisticConfig, StandardScaler, logistic_regression,
};

// Toy: one feature
let x = array![[0.0], [0.2], [0.8], [1.0]];
let y = array![0.0, 0.0, 1.0, 1.0];
let (scaler, xs) = StandardScaler::fit_transform(&x);
let model = logistic_regression(&xs, &y, &LogisticConfig {
    max_iter: 5000,
    learning_rate: 0.4,
    ..Default::default()
});

// --- export (print → paste into target) ---
println!("// StandardScaler");
println!("mean  = {:?}", scaler.mean.to_vec());
println!("scale = {:?}", scaler.scale.to_vec());
println!("// LogisticModel");
println!("intercept = {}", model.intercept);
println!("beta = {:?}", model.coefficients.to_vec());
```

### Example: dump OVR multiclass

```rust
// after logistic_regression_ovr(...)
for (cls, m) in model.classes.iter().zip(model.models.iter()) {
    println!("class {cls}: intercept={} beta={:?}", m.intercept, m.coefficients.to_vec());
}
```

### Example: dump rule list

```rust
// RuleListClassifier implements Display
println!("{clf}");
// Or walk clf.rules / conditions in code and emit JSON (see §6).
```

### Optional JSON shape (hand-maintained)

```json
{
  "version": 1,
  "features": ["hr_bpm", "rmssd_ms"],
  "scaled": true,
  "scaler": {
    "mean": [90.0, 40.0],
    "scale": [15.0, 12.0]
  },
  "model": "logistic_binary",
  "intercept": -0.5,
  "beta": [1.2, -0.8],
  "classes": { "0": "normal", "1": "elevated" }
}
```

Keep this file in app assets or firmware headers. Version it when features change.

---

## 3. Shared math (all platforms)

### Standardize

```text
z[j] = (x[j] - mean[j]) / scale[j]
```

### Sigmoid

```text
σ(t) = 1 / (1 + exp(-t))   // use stable form if |t| is large
```

### Binary logistic

```text
logit = intercept + sum_j beta[j] * z[j]
p     = σ(logit)
yhat  = 1 if p >= 0.5 else 0
```

### Multiclass OVR logistic

```text
for each class k:
  logit_k = intercept_k + sum_j beta_k[j] * z[j]
  p_k     = σ(logit_k)
// renormalize
p_k ← p_k / sum_m p_m
yhat = classes[argmax_k p_k]
```

### LDA

Same linear form as OVR scores (no sigmoid required for hard labels):

```text
score_k = intercept_k + sum_j coef_k[j] * x[j]   // or z[j] if trained scaled
yhat    = classes[argmax_k score_k]
```

### Rule list (first match)

```text
for rule in rules (in order):
  if all conditions true for raw x:   // usually raw units, not scaled
    return rule.label
return default_label
```

**Important:** if the model was trained on **scaled** features, logistic/LDA must see **scaled** `z`. Rule lists are almost always written on **physical units** (HR, RMSSD) — do **not** scale those unless the rules were authored that way.

---

## 4. Embedded (C / bare metal / RTOS)

Constraints: no heap preferred, `float` or `double`, fixed array sizes.

### Header with exported constants

```c
/* model_params.h — generated or pasted from Rust printout */
#ifndef MODEL_PARAMS_H
#define MODEL_PARAMS_H

#define N_FEATURES 2

static const float SCALER_MEAN[N_FEATURES]  = { 90.0f, 40.0f };
static const float SCALER_SCALE[N_FEATURES] = { 15.0f, 12.0f };

static const float LOG_INTERCEPT = -0.50f;
static const float LOG_BETA[N_FEATURES] = { 1.20f, -0.80f };

#endif
```

### Inference

```c
#include <math.h>
#include "model_params.h"

static float sigmoidf(float t) {
    if (t >= 0.0f) {
        float e = expf(-t);
        return 1.0f / (1.0f + e);
    } else {
        float e = expf(t);
        return e / (1.0f + e);
    }
}

/* x: raw features, length N_FEATURES */
float logistic_predict_proba(const float *x) {
    float logit = LOG_INTERCEPT;
    for (int j = 0; j < N_FEATURES; j++) {
        float z = (x[j] - SCALER_MEAN[j]) / SCALER_SCALE[j];
        logit += LOG_BETA[j] * z;
    }
    return sigmoidf(logit);
}

int logistic_predict(const float *x) {
    return logistic_predict_proba(x) >= 0.5f ? 1 : 0;
}
```

### Rule list on MCU

```c
typedef enum { OP_GE, OP_LT } Op;

typedef struct {
    int feature;
    Op op;
    float thr;
    int label;
} Rule;

/* Example: if HR >= 160 AND RMSSD < 20 → class 1; else 0 */
static int rule_predict(const float *x) {
    if (x[0] >= 160.0f && x[1] < 20.0f) return 1;
    if (x[0] < 55.0f && x[1] < 25.0f) return 2;
    if (x[0] >= 140.0f) return 1;
    return 0; /* default */
}
```

Tips:

- Prefer `float` on Cortex-M4F+; measure stack.  
- Avoid `malloc` in the hot path.  
- Keep a golden-vector test in firmware CI (same `x` as Rust).  
- Arduino / ESP-IDF: same C code as a `.c` / `.h` pair.

---

## 5. Mobile

### iOS (Swift)

```swift
import Foundation

struct StandardScaler {
    let mean: [Double]
    let scale: [Double]

    func transform(_ x: [Double]) -> [Double] {
        zip(x, zip(mean, scale)).map { xi, ms in
            let (m, s) = ms
            return (xi - m) / s
        }
    }
}

struct BinaryLogistic {
    let intercept: Double
    let beta: [Double]
    let scaler: StandardScaler?

    private func sigmoid(_ t: Double) -> Double {
        if t >= 0 {
            let e = exp(-t)
            return 1 / (1 + e)
        } else {
            let e = exp(t)
            return e / (1 + e)
        }
    }

    /// Raw features in the same order as training.
    func predictProba(_ x: [Double]) -> Double {
        let z = scaler?.transform(x) ?? x
        var logit = intercept
        for j in 0..<beta.count {
            logit += beta[j] * z[j]
        }
        return sigmoid(logit)
    }

    func predict(_ x: [Double], threshold: Double = 0.5) -> Int {
        predictProba(x) >= threshold ? 1 : 0
    }
}

// Load from bundle JSON or hardcode after export:
let model = BinaryLogistic(
    intercept: -0.5,
    beta: [1.2, -0.8],
    scaler: StandardScaler(mean: [90, 40], scale: [15, 12])
)
let p = model.predictProba([100, 35])
```

Ship parameters as `ModelParams.json` in the app bundle and decode with `Codable`, or embed as Swift constants for offline-only apps.

### Android (Kotlin)

```kotlin
data class StandardScaler(
    val mean: DoubleArray,
    val scale: DoubleArray,
) {
    fun transform(x: DoubleArray): DoubleArray =
        DoubleArray(x.size) { j -> (x[j] - mean[j]) / scale[j] }
}

class BinaryLogistic(
    private val intercept: Double,
    private val beta: DoubleArray,
    private val scaler: StandardScaler? = null,
) {
    private fun sigmoid(t: Double): Double =
        if (t >= 0) {
            val e = kotlin.math.exp(-t)
            1.0 / (1.0 + e)
        } else {
            val e = kotlin.math.exp(t)
            e / (1.0 + e)
        }

    fun predictProba(x: DoubleArray): Double {
        val z = scaler?.transform(x) ?: x
        var logit = intercept
        for (j in beta.indices) logit += beta[j] * z[j]
        return sigmoid(logit)
    }

    fun predict(x: DoubleArray, threshold: Double = 0.5): Int =
        if (predictProba(x) >= threshold) 1 else 0
}

// assets/model_params.json → parse, or:
val model = BinaryLogistic(
    intercept = -0.5,
    beta = doubleArrayOf(1.2, -0.8),
    scaler = StandardScaler(
        mean = doubleArrayOf(90.0, 40.0),
        scale = doubleArrayOf(15.0, 12.0),
    ),
)
```

### OVR multiclass (Swift sketch)

```swift
struct OvrLogistic {
    let classes: [Int]
    let intercepts: [Double]   // K
    let betas: [[Double]]      // K × p
    let scaler: StandardScaler?

    func predictProba(_ x: [Double]) -> [Double] {
        let z = scaler?.transform(x) ?? x
        var p = [Double](repeating: 0, count: classes.count)
        var sum = 0.0
        for k in 0..<classes.count {
            var logit = intercepts[k]
            for j in 0..<betas[k].count { logit += betas[k][j] * z[j] }
            // stable sigmoid
            let pk = logit >= 0
                ? 1 / (1 + exp(-logit))
                : exp(logit) / (1 + exp(logit))
            p[k] = pk
            sum += pk
        }
        return p.map { $0 / sum }
    }

    func predict(_ x: [Double]) -> Int {
        let p = predictProba(x)
        let k = p.enumerated().max(by: { $0.element < $1.element })!.offset
        return classes[k]
    }
}
```

Same structure ports directly to Kotlin.

### Mobile packaging tips

- Put JSON under `assets/` (Android) or app bundle (iOS).  
- Run inference on a background queue if batching; single-sample predict is cheap.  
- Do **not** call a remote API unless you want server-side models — this path is fully on-device.  
- For Flutter/React Native: implement once in Dart/TS or call platform channels into the Swift/Kotlin snippets above.

---

## 6. Web (browser / Node)

Vanilla TypeScript / JavaScript — no ML framework required.

```ts
export type Scaler = { mean: number[]; scale: number[] };

export function standardize(x: number[], s: Scaler): number[] {
  return x.map((xi, j) => (xi - s.mean[j]) / s.scale[j]);
}

export function sigmoid(t: number): number {
  if (t >= 0) {
    const e = Math.exp(-t);
    return 1 / (1 + e);
  }
  const e = Math.exp(t);
  return e / (1 + e);
}

export type BinaryLogisticParams = {
  intercept: number;
  beta: number[];
  scaler?: Scaler;
};

export function logisticPredictProba(
  x: number[],
  m: BinaryLogisticParams,
): number {
  const z = m.scaler ? standardize(x, m.scaler) : x;
  let logit = m.intercept;
  for (let j = 0; j < m.beta.length; j++) logit += m.beta[j] * z[j];
  return sigmoid(logit);
}

export function logisticPredict(
  x: number[],
  m: BinaryLogisticParams,
  thr = 0.5,
): number {
  return logisticPredictProba(x, m) >= thr ? 1 : 0;
}

// Load export JSON
// const params = await fetch('/models/vitals_logistic.json').then(r => r.json());
```

### Rule list on the web

```ts
type Op = "lt" | "le" | "gt" | "ge";
type Cond = { feature: number; op: Op; threshold: number };
type Rule = { conditions: Cond[]; label: number; name?: string };

function condOk(x: number[], c: Cond): boolean {
  const v = x[c.feature];
  switch (c.op) {
    case "lt": return v < c.threshold;
    case "le": return v <= c.threshold;
    case "gt": return v > c.threshold;
    case "ge": return v >= c.threshold;
  }
}

export function ruleListPredict(
  x: number[],
  rules: Rule[],
  defaultLabel: number,
): number {
  for (const r of rules) {
    if (r.conditions.every((c) => condOk(x, c))) return r.label;
  }
  return defaultLabel;
}
```

### Bundling

- Static `public/models/*.json` in Vite/Next/etc.  
- Or inline constants for offline PWAs.  
- WebAssembly: only needed if you want to run **full** Rust training in the browser; for inference, JS is enough and smaller.

---

## 7. Cross-platform verification

1. Pick a raw feature vector `x*`.  
2. In Rust: scale (if used) → `predict_proba` / `predict`.  
3. On target: same `x*` → compare `p` within ~1e-5 (float) or ~1e-6 (double).  
4. Fail CI if drift exceeds tolerance after a model update.

Example parity table:

| Platform | p(y=1) for x=[100, 35] |
|----------|-------------------------|
| Rust | 0.6123 |
| C MCU | 0.6123 |
| iOS | 0.6123 |
| Android | 0.6123 |
| Web | 0.6123 |

---

## 8. Security and product notes

- Exported weights are **not secret**; treat them as reverse-engineerable.  
- For regulated use, version models (`model_id`, `trained_at`, feature schema).  
- Rules are easiest to audit; OVR logistic is a good middle ground; avoid k-NN on device.  
- Retrain when sensors or units change; re-export scaler + model together.

---

## 9. Related SymWorx types

| Rust type | Module |
|-----------|--------|
| `StandardScaler` | `symworx_stats::preprocess` |
| `LogisticModel` / `logistic_regression` | `symworx_stats::logistic` |
| `MulticlassLogisticModel` / `logistic_regression_ovr` | `symworx_stats::logistic` |
| `LdaModel` / `lda` | `symworx_stats::lda` |
| `RuleListClassifier` | `symworx_stats::rules` |
| `GaussianNb` | `symworx_stats::naive_bayes` |

Demos that produce printable parameters:

```bash
cargo run -p symworx-stats --example logistic_regression_demo
cargo run -p symworx-stats --example multiclass_logistic_demo
cargo run -p symworx-stats --example rule_list_demo
cargo run -p symworx-stats --example lda_demo --features linalg
```

---

*Last updated: export guide for embedded C, iOS (Swift), Android (Kotlin), and web (TypeScript).*
