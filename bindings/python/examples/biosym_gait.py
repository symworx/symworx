"""
Simple Python example for symworx-biosym gait functionality.

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

# You can also construct and use more advanced objects
print("\nDemo complete.")
