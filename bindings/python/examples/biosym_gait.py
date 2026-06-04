"""
Simple Python example for symworx-biosym gait functionality (params + data analysis).

Usage (after maturin develop):
    python bindings/python/examples/biosym_gait.py
"""

import symworx_biosym as biosym

print("=== BioSym Gait Python Demo ===\n")

params = biosym.GaitParams()
print("Default GaitParams:")
print(f"  height       : {params.height:.2f} m")
print(f"  walking_speed: {params.walking_speed:.2f} m/s")
print(f"  step_length  : {params.step_length:.2f} m")

# GaitData + analysis (Phase 1 parity)
print("\nGaitData analysis demo:")
data = biosym.GaitData(100.0)
data.stride_times = [0.0, 1.15, 2.28, 3.41]
ints = data.calculate_stride_intervals()
print(f"  stride_intervals: {ints}")
lens = data.calculate_stride_length(1.3)
print(f"  stride_lengths (speed=1.3): {lens}")
print(f"  cadence: {data.calculate_cadence()}")
stats = data.to_gait_stats(1.3)
print(f"  GaitStats mean_stride_time: {stats.mean_stride_time_s:.3f}s, cadence: {stats.cadence_steps_min}")
print(f"  symmetry: {data.calculate_symmetry()}")

# New detect/analyze from signal (Phase 2)
print("\nSignal-based detection + analysis:")
sig = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0]
a = biosym.analyze_gait_signal(sig, 10.0)
print(f"  detected peaks: {len(a.peak_times)}, mean interval ~{a.stats.mean_stride_time_s:.2f}s")

print("\nDemo complete.")
