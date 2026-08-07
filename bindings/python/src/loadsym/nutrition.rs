// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

use pyo3::prelude::*;
use symworx_loadsym::nutrition::{
    ActivityLevel,
    BmrConfig,
    DeficitLevel,
    DeficitStrategy,
    Gender,
    calculate_bmi,
    calculate_bmr,
    calculate_calorie_targets,
    calculate_deficit,
    calculate_deficit_from_active,
    calculate_tdee,
    calculate_weightloss,
};

// ==========================================================
// Bodycomposition
// ==========================================================

#[pyclass]
#[derive(Clone, Copy)]
pub struct PyGender {
    pub inner: Gender,
}

#[pymethods]
impl PyGender {
    #[classattr]
    pub const MALE: Self = Self { inner: Gender::Male };
    #[classattr]
    pub const FEMALE: Self = Self { inner: Gender::Female };
    pub fn __repr__(&self) -> String {
        format!("Gender::{:?}", self.inner)
    }
}

#[pyclass]
#[derive(Clone, Copy)]
pub struct PyActivityLevel {
    pub inner: ActivityLevel,
}

#[pymethods]
impl PyActivityLevel {
    // --- Python enum-like constants ---
    #[classattr]
    pub const SEDENTARY: Self = Self {
        inner: ActivityLevel::Sedentary,
    };
    #[classattr]
    pub const LIGHT: Self = Self {
        inner: ActivityLevel::Light,
    };
    #[classattr]
    pub const MODERATE: Self = Self {
        inner: ActivityLevel::Moderate,
    };
    #[classattr]
    pub const ACTIVE: Self = Self {
        inner: ActivityLevel::Active,
    };
    #[classattr]
    pub const VERY_ACTIVE: Self = Self {
        inner: ActivityLevel::VeryActive,
    };
    #[classattr]
    pub const EXTRA_ACTIVE: Self = Self {
        inner: ActivityLevel::ExtraActive,
    };
    // --- repr for debugging ---
    pub fn __repr__(&self) -> String {
        format!("ActivityLevel::{:?}", self.inner)
    }
}

#[pyclass]
#[derive(Clone, Copy)]
pub struct PyDeficitLevel {
    pub inner: DeficitLevel,
}

#[pymethods]
impl PyDeficitLevel {
    #[classattr]
    pub const LIGHT: Self = Self {
        inner: DeficitLevel::Light,
    };
    #[classattr]
    pub const MILD: Self = Self {
        inner: DeficitLevel::Mild,
    };
    #[classattr]
    pub const MODERATE: Self = Self {
        inner: DeficitLevel::Moderate,
    };
    #[classattr]
    pub const AGGRESSIVE: Self = Self {
        inner: DeficitLevel::Aggressive,
    };
    #[classattr]
    pub const EXTREME: Self = Self {
        inner: DeficitLevel::Extreme,
    };

    pub fn __repr__(&self) -> String {
        format!("DeficitLevel::{:?}", self.inner)
    }
}

#[pyclass]
#[derive(Clone, Copy)]
pub struct PyDeficitStrategy {
    pub inner: DeficitStrategy,
}

#[pymethods]
impl PyDeficitStrategy {
    #[classattr]
    pub const CALORIC_RESTRICTION: Self = Self {
        inner: DeficitStrategy::CaloricRestriction,
    };
    #[classattr]
    pub const ACTIVITY_INCREASE: Self = Self {
        inner: DeficitStrategy::ActivityIncrease,
    };
    #[classattr]
    pub const BALANCED: Self = Self {
        inner: DeficitStrategy::Balanced,
    };

    pub fn __repr__(&self) -> String {
        format!("DeficitStrategy::{:?}", self.inner)
    }
}

#[pyfunction(name = "calculate_bmr")]
pub fn py_calculate_bmr(weight_kg: f64, height_m: f64, age_years: f64, is_male: bool) -> PyResult<f64> {
    let gender = if is_male { Gender::Male } else { Gender::Female };
    Ok(calculate_bmr(
        weight_kg,
        height_m,
        age_years,
        gender,
        BmrConfig::default(),
    ))
}

#[pyfunction(name = "calculate_bmi")]
pub fn py_calculate_bmi(weight_kg: f64, height_m: f64) -> PyResult<f64> {
    Ok(calculate_bmi(weight_kg, height_m))
}

#[pyfunction(name = "calculate_tdee")]
pub fn py_calculate_tdee(bmr: f64, activity: &str) -> PyResult<f64> {
    let level = match activity.to_lowercase().as_str() {
        "sedentary" => ActivityLevel::Sedentary,
        "light" => ActivityLevel::Light,
        "moderate" => ActivityLevel::Moderate,
        "active" => ActivityLevel::Active,
        "very active" => ActivityLevel::VeryActive,
        "extra active" => ActivityLevel::ExtraActive,
        _ => ActivityLevel::Moderate,
    };

    Ok(calculate_tdee(bmr, level))
}

#[pyfunction(name = "calculate_deficit")]
pub fn py_calculate_deficit(bmr: f64, tdee: f64, deficit_level: &str) -> PyResult<f64> {
    let level = match deficit_level.to_lowercase().as_str() {
        "light" => DeficitLevel::Light,
        "mild" => DeficitLevel::Mild,
        "moderate" => DeficitLevel::Moderate,
        "aggressive" => DeficitLevel::Aggressive,
        "extreme" => DeficitLevel::Extreme,
        _ => DeficitLevel::Moderate,
    };
    Ok(calculate_deficit(bmr, tdee, level))
}

#[pyfunction(name = "calculate_deficit_from_active")]
pub fn py_calculate_deficit_from_active(bmr: f64, tdee: f64, deficit_level: &str) -> PyResult<f64> {
    let level = match deficit_level.to_lowercase().as_str() {
        "light" => DeficitLevel::Light,
        "mild" => DeficitLevel::Mild,
        "moderate" => DeficitLevel::Moderate,
        "aggressive" => DeficitLevel::Aggressive,
        "extreme" => DeficitLevel::Extreme,
        _ => DeficitLevel::Moderate,
    };
    Ok(calculate_deficit_from_active(bmr, tdee, level))
}

#[pyfunction(name = "calculate_calorie_targets")]
pub fn py_calculate_calorie_targets(tdee: f64, bmr: f64, deficit: f64, strategy: &str) -> PyResult<(f64, f64)> {
    let strat = match strategy.to_lowercase().as_str() {
        "caloric_restriction" | "restriction" => DeficitStrategy::CaloricRestriction,
        "activity_increase" | "activity" => DeficitStrategy::ActivityIncrease,
        "balanced" => DeficitStrategy::Balanced,
        _ => DeficitStrategy::Balanced,
    };
    Ok(calculate_calorie_targets(tdee, bmr, deficit, strat))
}

#[pyfunction(name = "calculate_weightloss")]
pub fn py_calculate_weightloss(
    age_years: f64,
    is_male: bool,
    height_m: f64,
    starting_weight_kg: f64,
    target_weight_kg: f64,
    activity: &str,
    deficit_level: &str,
    strategy: &str,
) -> PyResult<PyObject> {
    let act_level = match activity.to_lowercase().as_str() {
        "sedentary" => ActivityLevel::Sedentary,
        "light" => ActivityLevel::Light,
        "moderate" => ActivityLevel::Moderate,
        "active" => ActivityLevel::Active,
        "very active" => ActivityLevel::VeryActive,
        "extra active" => ActivityLevel::ExtraActive,
        _ => ActivityLevel::Moderate,
    };

    let def_level = match deficit_level.to_lowercase().as_str() {
        "light" => DeficitLevel::Light,
        "mild" => DeficitLevel::Mild,
        "moderate" => DeficitLevel::Moderate,
        "aggressive" => DeficitLevel::Aggressive,
        "extreme" => DeficitLevel::Extreme,
        _ => DeficitLevel::Moderate,
    };

    let strat = match strategy.to_lowercase().as_str() {
        "caloric_restriction" | "restriction" => DeficitStrategy::CaloricRestriction,
        "activity_increase" | "activity" => DeficitStrategy::ActivityIncrease,
        "balanced" => DeficitStrategy::Balanced,
        _ => DeficitStrategy::Balanced,
    };

    let gender = if is_male { Gender::Male } else { Gender::Female };
    let model = calculate_weightloss(
        age_years,
        gender,
        height_m,
        starting_weight_kg,
        target_weight_kg,
        act_level,
        def_level,
        strat,
        BmrConfig::default(),
    );

    // Convert to a Python dict for easy consumption
    Python::with_gil(|py| {
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("gender", format!("{:?}", model.gender))?;
        dict.set_item("deficit_level", format!("{:?}", model.deficit_level))?;
        dict.set_item("deficit_strategy", format!("{:?}", model.deficit_strategy))?;
        dict.set_item(
            "bmr_config_obesity",
            format!("{:?}", model.bmr_config.obesity_adjustment),
        )?;
        dict.set_item("activity_level", format!("{:?}", model.activity_level))?;
        dict.set_item("age_years", model.age_years)?;
        dict.set_item("height_m", model.height_m)?;
        dict.set_item("starting_weight_kg", model.starting_weight_kg)?;
        dict.set_item("target_weight_kg", model.target_weight_kg)?;
        dict.set_item("week", model.week)?;
        dict.set_item("weight_kg", model.weight_kg)?;
        dict.set_item("bmi", model.bmi)?;
        dict.set_item("weekly_deficit_kcal", model.weekly_deficit_kcal)?;
        dict.set_item("total_deficit_kcal", model.total_deficit_kcal)?;
        Ok(dict.into())
    })
}

// ==========================================================
// PYTHON REGISTER
// ==========================================================
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Enums / Classes
    m.add_class::<PyGender>()?;
    m.add_class::<PyActivityLevel>()?;
    m.add_class::<PyDeficitLevel>()?;
    m.add_class::<PyDeficitStrategy>()?;

    // Functions
    m.add_function(wrap_pyfunction!(py_calculate_bmr, m)?)?;
    m.add_function(wrap_pyfunction!(py_calculate_bmi, m)?)?;
    m.add_function(wrap_pyfunction!(py_calculate_tdee, m)?)?;
    m.add_function(wrap_pyfunction!(py_calculate_deficit, m)?)?;
    m.add_function(wrap_pyfunction!(py_calculate_deficit_from_active, m)?)?;
    m.add_function(wrap_pyfunction!(py_calculate_calorie_targets, m)?)?;
    m.add_function(wrap_pyfunction!(py_calculate_weightloss, m)?)?;

    Ok(())
}
