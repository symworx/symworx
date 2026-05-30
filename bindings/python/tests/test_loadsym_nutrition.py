"""
Minimal tests for the symworx-loadsym nutrition Python bindings.

Run after:
    cd bindings/python
    maturin develop --manifest-path ../crates/symworx-loadsym/Cargo.toml -m pyproject-loadsym.toml

Then:
    python -m pytest tests/test_loadsym_nutrition.py -q
    # or simply: python tests/test_loadsym_nutrition.py
"""

import sys
from pathlib import Path

# Make sure the package can be found when running directly
sys.path.insert(0, str(Path(__file__).parent.parent / "symworx"))

try:
    import symworx_loadsym as loadsym
except ImportError:
    # Fallback when running inside the installed package layout
    from symworx import loadsym


def test_calculate_bmi():
    bmi = loadsym.nutrition.calculate_bmi(70.0, 1.75)
    assert 22.8 < bmi < 22.9


def test_calculate_bmr():
    bmr = loadsym.nutrition.calculate_bmr(70.0, 1.75, 30.0, True)
    assert 1600 < bmr < 1700


def test_calculate_calorie_targets():
    intake, activity = loadsym.nutrition.calculate_calorie_targets(
        2500.0, 1600.0, 500.0, "balanced"
    )
    assert 2249 < intake < 2251
    assert 1149 < activity < 1151


def test_calculate_weightloss_smoke():
    model = loadsym.nutrition.calculate_weightloss(
        age_years=30.0,
        is_male=True,
        height_m=1.75,
        starting_weight_kg=82.0,
        target_weight_kg=78.0,
        activity="moderate",
        deficit_level="moderate",
        strategy="balanced",
    )
    assert len(model["week"]) >= 1
    assert len(model["weight_kg"]) == len(model["week"])
    # Final BMI should be plausible
    final_bmi = model["bmi"][-1]
    assert 20.0 < final_bmi < 30.0


if __name__ == "__main__":
    # Allow running as a plain script
    test_calculate_bmi()
    test_calculate_bmr()
    test_calculate_calorie_targets()
    test_calculate_weightloss_smoke()
    print("All loadsym nutrition Python tests passed!")
