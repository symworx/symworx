"""
Smoke test for symworx-biosym gait (Python bindings) including analysis parity.

Run after:
    maturin develop --manifest-path bindings/python/Cargo.toml
"""

from symworx import biosym


def test_gait_params_defaults():
    params = biosym.GaitParams()
    assert hasattr(params, "height")
    assert params.height > 1.0  # meters
    # Also available under biomechanics (submodule path)
    assert hasattr(biosym.biomechanics, "GaitParams")


def test_gait_data_calcs_and_stats():
    data = biosym.GaitData(100.0)
    data.stride_times = [0.0, 1.0, 2.0, 3.0]
    intervals = data.calculate_stride_intervals()
    assert intervals is not None and len(intervals) == 3
    lengths = data.calculate_stride_length(1.3)
    assert lengths is not None and len(lengths) == 3
    assert abs(lengths[0] - 1.3) < 1e-9
    cad = data.calculate_cadence()
    assert cad is not None and abs(cad - 120.0) < 1e-6
    data.calculate_step_times()
    assert data.left_step_times is not None
    step_ints = data.calculate_step_intervals()
    assert step_ints is not None
    sym = data.calculate_symmetry()
    assert sym is not None and sym < 1e-6  # symmetric input
    stats = data.to_gait_stats(1.3)
    assert stats.n_strides == 3
    assert abs(stats.mean_stride_time_s - 1.0) < 1e-9
    assert stats.cadence_steps_min is not None and abs(stats.cadence_steps_min - 120.0) < 1e-6
    assert stats.mean_stride_length_m is not None and abs(stats.mean_stride_length_m - 1.3) < 1e-9


def test_gait_detect_and_analyze():
    # Long enough periodic signal for bandpass peak detection (matches Rust gait tests).
    import math

    fs = 100.0
    # ~1.2 Hz stride-like sinusoid over 10 s
    sig = [math.sin(2.0 * math.pi * 1.2 * (i / fs)) for i in range(int(10 * fs))]
    analysis = biosym.analyze_gait_signal(sig, fs)
    assert hasattr(analysis, "stats")
    assert hasattr(analysis, "peak_times")
    assert len(analysis.peak_times) >= 1
    assert analysis.stats.n_strides >= 1


if __name__ == "__main__":
    test_gait_params_defaults()
    test_gait_data_calcs_and_stats()
    test_gait_detect_and_analyze()
    print("biosym Python gait smoke test passed!")
