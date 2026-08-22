"""
Core statistics / classical ML smoke tests for the unified ``symworx`` package.

Run after:
    maturin develop --manifest-path bindings/python/Cargo.toml
"""

from symworx.core import statistics as st


def test_mean_median():
    assert st.mean([1.0, 2.0, 3.0]) == 2.0
    assert st.median([1.0, 2.0, 3.0]) == 2.0


def test_train_test_split_indices():
    plan = st.train_test_split(100, test_ratio=0.3, n_train_folds=5, shuffle=True, seed=7)
    assert plan.n == 100
    assert len(plan.train_idx) + len(plan.test_idx) == 100
    assert len(plan.folds) == 5
    assert set(plan.train_idx) | set(plan.test_idx) == set(range(100))
    fit0 = plan.fit_idx(0)
    val0 = plan.val_idx(0)
    assert len(fit0) + len(val0) == len(plan.train_idx)


def test_standard_scaler_and_logistic():
    x = [[0.0], [0.1], [0.2], [0.8], [0.9], [1.0]]
    y = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0]
    scaler, xs = st.standard_scaler_fit(x)
    assert len(scaler.mean) == 1
    model = st.logistic_regression(
        xs, y, max_iter=5000, learning_rate=0.5, l2=0.0, tol=1e-8
    )
    assert model.converged or model.iterations > 0
    proba = model.predict_proba(xs)
    assert len(proba) == 6
    assert proba[0] < proba[-1]
    pred = model.predict(xs, threshold=0.5)
    assert all(p in (0.0, 1.0) for p in pred)
    acc = model.accuracy(xs, y)
    assert acc >= 0.8


def test_logistic_ovr_and_metrics():
    x = [
        [0.0, 0.0],
        [0.1, 0.05],
        [0.05, 0.1],
        [4.0, 4.0],
        [4.1, 3.9],
        [3.9, 4.1],
        [0.0, 4.0],
        [0.1, 4.1],
        [-0.05, 3.9],
    ]
    y = [0, 0, 0, 1, 1, 1, 2, 2, 2]
    scaler, xs = st.standard_scaler_fit(x)
    model = st.logistic_regression_ovr(
        xs, y, max_iter=6000, learning_rate=0.35, l2=0.01, tol=1e-8
    )
    assert model.n_classes() == 3
    pred = model.predict(xs)
    assert len(pred) == len(y)
    rep = st.classification_report(y, pred, n_classes=3)
    assert rep["n"] == 9
    assert rep["accuracy"] >= 0.8
    proba = model.predict_proba(xs)
    auc = st.roc_auc_ovr(y, proba, classes=model.classes)
    assert auc == auc  # not NaN
    assert auc > 0.7


def test_roc_auc_perfect():
    y = [0, 0, 1, 1]
    s = [0.1, 0.2, 0.8, 0.9]
    assert abs(st.roc_auc(y, s) - 1.0) < 1e-9


def test_lmer_random_intercept():
    y, x, groups = st.simulate_random_intercept(
        n_groups=40, n_per_group=5, intercept=2.0, coefficients=[1.5],
        sigma2=1.0, sigma_u2=4.0, seed=42,
    )
    fit = st.lmer(y, x, groups, name="subject", kind="intercept")
    assert fit.n == 200
    assert fit.n_groups() == 40
    assert abs(fit.intercept - 2.0) < 0.6
    assert abs(fit.coefficients[0] - 1.5) < 0.4
    assert fit.sigma_u2() > 1.0
    yhat = fit.predict(x)
    assert len(yhat) == len(y)
    yhat_g = fit.predict_conditional(x, groups)
    assert len(yhat_g) == len(y)
    assert fit.summary()


def test_gaussian_nb():
    x = [
        [0.0, 0.0],
        [0.1, 0.0],
        [0.0, 0.1],
        [5.0, 5.0],
        [5.1, 5.0],
        [5.0, 5.1],
    ]
    y = [0, 0, 0, 1, 1, 1]
    clf = st.gaussian_nb(x, y)
    pred = clf.predict(x)
    assert pred == y
    assert abs(st.accuracy(y, pred) - 1.0) < 1e-12
