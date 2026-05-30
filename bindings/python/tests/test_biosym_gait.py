"""
Very light smoke test for symworx-biosym gait (Python bindings).

Run after installing the package via maturin.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "symworx"))

try:
    import symworx_biosym as biosym
except ImportError:
    from symworx import biosym


def test_gait_params_defaults():
    params = biosym.GaitParams()
    # The Rust side should have run with_defaults internally in some paths,
    # but we just check that the object is constructible and has expected fields.
    assert hasattr(params, "height")
    assert params.height > 1.0  # meters


if __name__ == "__main__":
    test_gait_params_defaults()
    print("biosym Python smoke test passed!")
