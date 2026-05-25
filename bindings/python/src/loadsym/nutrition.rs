// Copyright (c) 2026 SymWorx. All rights reserved.

use pyo3::prelude::*;

use symworx_loadsym::nutrition::{ActivityLevel, calculate_bmr, calculate_tdee};

// ==========================================================
// Bodycomposition 
// ==========================================================

#[pyclass]
#[derive(Clone, Copy)]
pub struct PyActivityLevel {
    pub inner: ActivityLevel,
}

#[pymethods]
impl PyActivityLevel {
    // --- Python enum-like constants ---
    #[classattr]
    pub const SEDENTARY: Self = Self { inner: ActivityLevel::Sedentary };
    #[classattr]
    pub const LIGHT: Self = Self { inner: ActivityLevel::Light };
    #[classattr]
    pub const MODERATE: Self = Self { inner: ActivityLevel::Moderate };
    #[classattr]
    pub const ACTIVE: Self = Self { inner: ActivityLevel::Active };
    #[classattr]
    pub const VERY_ACTIVE: Self = Self { inner: ActivityLevel::VeryActive };
    #[classattr]
    pub const EXTRA_ACTIVE: Self = Self { inner: ActivityLevel::ExtraActive };
    // --- repr for debugging ---
    pub fn __repr__(&self) -> String {
        format!("ActivityLevel::{:?}", self.inner)
    }
}

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

// ==========================================================
// PYTHON REGISTER
// ==========================================================
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {

    m.add_function(wrap_pyfunction!(py_calculate_bmr, m)?)?;
    m.add_function(wrap_pyfunction!(py_calculate_tdee, m)?)?;

    // TODO: add PyActivityLevel class
    
    Ok(())
}
