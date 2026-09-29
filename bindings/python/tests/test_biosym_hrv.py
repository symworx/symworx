"""
HRV bindings: SDNN and `compute_hrv_metrics` from biosym.

Run after:
    maturin develop --manifest-path bindings/python/Cargo.toml
"""

import math

from symworx.biosym import physiology as phys
from symworx.core import statistics as st


def test_compute_hrv_metrics_matches_sdnn():
    rr = [1.0, 2.0, 3.0]
    expected = math.sqrt(2.0 / 3.0)
    hrv = phys.compute_hrv_metrics(rr)
    assert abs(hrv.sdnn_sec - expected) < 1e-12
    assert abs(hrv.rmssd_sec - 1.0) < 1e-12
    assert abs(phys.sdnn(rr) - st.sdnn(rr)) < 1e-12


def test_compute_hrv_metrics_short_series():
    one = phys.compute_hrv_metrics([0.9])
    assert one.sdnn_sec is None
    assert one.rmssd_sec is None

    two = phys.compute_hrv_metrics([1.0, 3.0])
    assert two.rmssd_sec is None
    assert abs(two.sdnn_sec - 1.0) < 1e-12

    constant = phys.compute_hrv_metrics([0.8, 0.8, 0.8, 0.8])
    assert constant.sdnn_sec == 0.0
    assert constant.rmssd_sec == 0.0
