# Copyright (c) 2026 Nathaniel T. Berry
# Licensed under the Apache License, Version 2.0.

from math import isfinite

from symworx.core import processing as proc


def test_exponential_kernel_and_impulse_convolution():
    h = proc.exponential_kernel(10.0, 15.0, 4)
    assert len(h) == 4
    assert abs(h[0] - 1.0) < 1e-12
    c = proc.convolve([1.0, 0.0, 0.0, 0.0], [1.0, 0.5, 0.25])
    assert abs(c[0] - 1.0) < 1e-12
    assert abs(c[1] - 0.5) < 1e-12
    assert abs(c[2] - 0.25) < 1e-12


def test_gh_deconvolve_reconvolve_roundtrip():
    kernel = proc.exponential_kernel(10.0, 15.0, 24)
    secretion = [0.0] * 24
    secretion[6] = 2.0
    observed = proc.convolve(secretion, kernel)
    recovered = proc.gh_deconvolve(observed, kernel)
    recon = proc.gh_reconvolve(recovered, kernel)
    rmse = (sum((a - b) ** 2 for a, b in zip(observed, recon)) / len(observed)) ** 0.5
    assert rmse < 0.05
    assert recovered[6] == max(recovered)
    assert all(x >= 0.0 and isfinite(x) for x in recovered)
