#!/usr/bin/env python3
# Copyright (c) 2026 SymWorx
"""
Classical ML via Python bindings (split → scale → logistic OVR).

Run after:
    maturin develop --manifest-path bindings/python/Cargo.toml
    python bindings/python/examples/stats_logistic.py
"""

from __future__ import annotations

from symworx.core import statistics as st


def main() -> None:
    # Three blobs in 2D
    x: list[list[float]] = []
    y: list[int] = []
    for i in range(15):
        if i < 5:
            x.append([0.1 * i, 0.05 * i])
            y.append(0)
        elif i < 10:
            x.append([3.0 + 0.1 * (i - 5), 3.0 + 0.05 * (i - 5)])
            y.append(1)
        else:
            x.append([0.05 * (i - 10), 3.0 + 0.1 * (i - 10)])
            y.append(2)

    n = len(x)
    plan = st.train_test_split(n, test_ratio=0.3, n_train_folds=None, shuffle=True, seed=3)
    print(plan)

    def take(rows: list[list[float]], idx: list[int]) -> list[list[float]]:
        return [rows[i] for i in idx]

    def take1(labels: list[int], idx: list[int]) -> list[int]:
        return [labels[i] for i in idx]

    x_train = take(x, plan.train_idx)
    y_train = take1(y, plan.train_idx)
    x_test = take(x, plan.test_idx)
    y_test = take1(y, plan.test_idx)

    scaler, x_train_s = st.standard_scaler_fit(x_train)
    x_test_s = scaler.transform(x_test)

    model = st.logistic_regression_ovr(
        x_train_s,
        y_train,
        max_iter=8000,
        learning_rate=0.4,
        l2=0.05,
        tol=1e-8,
    )
    print(model)
    print("classes", model.classes)

    pred_te = model.predict(x_test_s)
    rep = st.classification_report(y_test, pred_te, n_classes=3)
    print("test report:", {k: rep[k] for k in ("accuracy", "macro_f1", "balanced_accuracy")})

    proba = model.predict_proba(x_test_s)
    auc = st.roc_auc_ovr(y_test, proba, classes=model.classes)
    print(f"test macro OVR ROC-AUC = {auc:.4f}")

    # Binary logistic on first feature only
    xb = [[row[0]] for row in x]
    yb = [0.0 if yi == 0 else 1.0 for yi in y]
    sc2, xb_s = st.standard_scaler_fit(xb)
    bin_m = st.logistic_regression(xb_s, yb, max_iter=5000, learning_rate=0.4)
    scores = bin_m.predict_proba(xb_s)
    yb_i = [int(v) for v in yb]
    print(f"binary in-sample ROC-AUC = {st.roc_auc(yb_i, scores):.4f}")
    _ = sc2


if __name__ == "__main__":
    main()
