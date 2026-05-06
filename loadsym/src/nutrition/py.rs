// loadsym/src/nutrition.py.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;

// ==========================================================
// Nutrition 
// ==========================================================
// Bodycomposition 
// ----------------------------------------------------------
#[pyfunction(name = "calculate_bmr")]
pub fn py_calculate_bmr(
    weight_kg: f64,
    height_cm: f64,
    age_years: f64,
    is_male: bool
) -> PyResult<f64> {
    Ok(calculate_bmr(weight_kg, height_cm, age_years, is_male))
}


#[pyfunction(name = "calculate_tdee")]
pub fn py_calculate_tdee(bmr: f64, activity: &str) -> PyResult<f64> {
    let level = match activity.to_lowercase().as_str() {
        "sedentary"    => ActivityLevel::Sedentary,
        "light"        => ActivityLevel::Light,
        "moderate"     => ActivityLevel::Moderate,
        "active"       => ActivityLevel::Active,
        "very active"  => ActivityLevel::VeryActive,
        "extra active" => ActivityLevel::ExtraActive,
        _              => ActivityLevel::Moderate,
    };

    Ok(calculate_tdee(bmr, level))
}
