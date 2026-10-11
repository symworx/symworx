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


def _lcg(n, seed):
    s = seed
    out = []
    for _ in range(n):
        s = (s * 6364136223846793005 + 1) & ((1 << 64) - 1)
        out.append(((s >> 33) & 0xFFFFFFFF) / 0xFFFFFFFF)
    return out


def test_acf_normalized_lag0_is_one():
    signal = [1.0, 2.0, 3.0, 4.0, 5.0]
    raw = st.acf(signal, False)
    norm = st.acf_normalized(signal, 4)
    assert abs(norm[0] - 1.0) < 1e-12
    assert all(abs(norm[k] - raw[k] / raw[0]) < 1e-12 for k in range(5))
    assert st.acf_normalized([1.0, 1.0, 1.0], 3) == []


def test_ccf_peaks_at_planted_lag():
    driver = _lcg(400, 3)
    k = 4
    target = [0.0] * len(driver)
    for t in range(k, len(driver)):
        target[t] = driver[t - k]
    curve = st.ccf(driver, target, 8)
    assert curve.index(max(curve)) == k
    assert curve[k] > 0.99
    assert st.ccf(driver, target[:-1], 2) == []


def test_relative_kmeans_discretizer_is_shift_invariant():
    values = [10.0, 20.0, 30.0, 40.0] * 20
    groups = [0] * len(values)
    disc = st.relative_kmeans_discretizer(values, groups, 3, seed=3)
    assert disc.n_bins == 3
    assert len(disc.cuts) == 2
    bins = disc.transform(values, groups)
    shifted = disc.transform_new_group([v + 250.0 for v in values])
    assert bins == shifted
