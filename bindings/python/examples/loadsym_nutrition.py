#!/usr/bin/env python3
# Copyright (c) 2026 PalEm Dynamics LLC
# Licensed under the Apache License, Version 2.0.
"""
Nutrition demo for the unified ``symworx`` package.

    maturin develop --manifest-path bindings/python/Cargo.toml
    python bindings/python/examples/loadsym_nutrition.py
"""

from symworx import loadsym

print("=== LoadSym Nutrition Python Demo ===\n")

weight = 78.5
height_m = 1.78
age = 32.0
is_male = True

bmr = loadsym.nutrition.calculate_bmr(weight, height_m, age, is_male)
bmi = loadsym.nutrition.calculate_bmi(weight, height_m)
tdee = loadsym.nutrition.calculate_tdee(bmr, "moderate")

print("Athlete profile:")
print(f"  Weight : {weight:.1f} kg")
print(f"  Height : {height_m:.2f} m (BMI = {bmi:.1f})")
print(f"  Age    : {age:.0f} years ({'male' if is_male else 'female'})")
print(f"  BMR    : {bmr:.0f} kcal")
print(f"  TDEE (moderate) : {tdee:.0f} kcal\n")

intake, activity = loadsym.nutrition.calculate_calorie_targets(
    tdee, bmr, 550.0, "balanced"
)
print("Target for ~550 kcal daily deficit (balanced):")
print(f"  Daily intake target   : {intake:.0f} kcal")
print(f"  Daily activity target : {activity:.0f} kcal\n")

print("Running weight loss simulation (moderate deficit)...")
model = loadsym.nutrition.calculate_weightloss(
    age_years=age,
    is_male=is_male,
    height_m=height_m,
    starting_weight_kg=weight,
    target_weight_kg=72.0,
    activity="moderate",
    deficit_level="moderate",
    strategy="balanced",
)

weeks = len(model["week"])
final_w = model["weight_kg"][-1]
final_bmi = model["bmi"][-1]

print(f"Simulation finished after {weeks} weeks")
print(f"  Final weight : {final_w:.1f} kg")
print(f"  Final BMI    : {final_bmi:.1f}")
print("\nDemo complete.")
