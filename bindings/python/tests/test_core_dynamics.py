"""
Dynamics lag curves: histogram AMI and cross-AMI.

Run after:
    maturin develop --manifest-path bindings/python/Cargo.toml
"""

from symworx.core import dynamics as dyn


def _lcg(n, seed):
    s = seed
    out = []
    for _ in range(n):
        s = (s * 6364136223846793005 + 1) & ((1 << 64) - 1)
        out.append(((s >> 33) & 0xFFFFFFFF) / 0xFFFFFFFF)
    return out


def test_mutual_information_identical_beats_independent():
    x = _lcg(400, 1)
    y = _lcg(400, 2)
    same = dyn.mutual_information(x, x)
    indep = dyn.mutual_information(x, y)
    assert same > 1.0
    assert indep < 0.2
    assert dyn.mutual_information(x, y[:-1]) == 0.0
    assert dyn.DEFAULT_AMI_BINS == 8


def test_cross_ami_peaks_at_planted_lag():
    driver = _lcg(500, 9)
    k = 3
    target = [0.0] * len(driver)
    for t in range(k, len(driver)):
        target[t] = driver[t - k]
    curve = dyn.cross_ami(driver, target, 8)
    assert curve.index(max(curve)) == k


def test_ami_lag0_matches_self_mi():
    x = _lcg(300, 11)
    auto = dyn.ami(x, 5)
    assert abs(auto[0] - dyn.mutual_information(x, x)) < 1e-12
    assert dyn.ami(x, 5) == dyn.cross_ami(x, x, 5)


def test_mutual_information_discrete_labels():
    labels = [0, 1, 0, 1, 2, 2, 0, 1]
    assert dyn.mutual_information_discrete(labels, labels) > 0.0
    assert dyn.mutual_information_discrete(labels, labels[:-1]) == 0.0


def test_first_local_minimum():
    assert dyn.first_local_minimum([1.0, 0.5, 0.2, 0.4, 0.3]) == 2
    assert dyn.first_local_minimum([1.0, 0.5, 0.2]) is None


def _drive_series(n, seed):
    s = seed
    x = []
    y = []
    x_prev = 0.0
    for _ in range(n):
        s = (s * 6364136223846793005 + 1) & ((1 << 64) - 1)
        xi = ((s >> 33) & 0xFFFFFFFF) / 0xFFFFFFFF - 0.5
        s = (s * 6364136223846793005 + 1) & ((1 << 64) - 1)
        noise = ((s >> 33) & 0xFFFFFFFF) / 0xFFFFFFFF - 0.5
        yi = 0.85 * x_prev + 0.15 * noise
        x.append(xi)
        y.append(yi)
        x_prev = xi
    return x, y


def test_transfer_entropy_coupled_source_predicts_target():
    x, y = _drive_series(400, 7)
    cfg = dyn.TeConfig()
    te_xy = dyn.transfer_entropy_with(x, y, cfg)
    te_yx = dyn.transfer_entropy_with(y, x, cfg)
    assert te_xy > 0.0
    assert te_xy > te_yx
    assert dyn.transfer_entropy(x, y) == te_xy


def test_transfer_entropy_short_or_constant_is_zero():
    assert dyn.transfer_entropy([0.1, 0.2], [0.3, 0.4]) == 0.0
    assert dyn.transfer_entropy([1.0] * 80, [2.0] * 80) == 0.0


def test_transfer_entropy_conditional_keeps_the_driver():
    x, y = _drive_series(400, 11)
    z = _lcg(len(y), 123)
    z = [v - 0.5 for v in z]
    cfg = dyn.TeConfig()
    te_x_given_z = dyn.transfer_entropy_conditional([z], [x], y, cfg)
    te_z_given_x = dyn.transfer_entropy_conditional([x], [z], y, cfg)
    assert te_x_given_z > te_z_given_x
    joint = dyn.transfer_entropy_mv([x, z], y, cfg)
    assert joint > 0.0


def test_transfer_entropy_discrete_coupled_labels():
    n = 400
    s = 7
    source = []
    target = []
    prev = 0
    for _ in range(n):
        s = (s * 6364136223846793005 + 1) & ((1 << 64) - 1)
        xi = s % 4
        s = (s * 6364136223846793005 + 1) & ((1 << 64) - 1)
        if s % 10 < 8:
            yi = prev
        else:
            s = (s * 6364136223846793005 + 1) & ((1 << 64) - 1)
            yi = s % 4
        source.append(xi)
        target.append(yi)
        prev = xi
    te_xy = dyn.transfer_entropy_discrete(source, target)
    te_yx = dyn.transfer_entropy_discrete(target, source)
    assert te_xy > 0.0
    assert te_xy > te_yx
