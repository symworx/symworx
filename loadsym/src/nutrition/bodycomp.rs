// loadsym/src/nutrition/bodycomp.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;

// ==========================================================
// Constants
// ==========================================================

/// Calculate basal metabolic rate (BMR) using the Mifflin-St Jeor Equation
///
/// # Arguments
/// - `weight_kg`: Weight in kilograms
/// - `height_cm`: Height in centimeters
/// - `age_years`: Age in years
/// - `is_male`: True
///
/// # Returns
/// - BMR in calories/day
pub fn calculate_bmr(weight_kg: f64, height_cm: f64, age_years: f64, is_male: bool) -> f64 {
    if is_male {
        10.0 * weight_kg + 6.25 * height_cm - 5.0 * age_years + 5.0
    } else {
        10.0 * weight_kg + 6.25 * height_cm - 5.0 * age_years - 161.0
    }
}

/// Activity level enum for TDEE
#[derive(Debug, Clone, Copy)]
pub enum ActivityLevel {
    Sedentary,
    Light,
    Moderate,
    Active,
    VeryActive,
    ExtraActive,
}

impl ActivityLevel {
    pub fn factor(self) -> f64 {
        match self {
            ActivityLevel::Sedentary   => 1.20,
            ActivityLevel::Light       => 1.375,
            ActivityLevel::Moderate    => 1.55,
            ActivityLevel::Active      => 1.725,
            ActivityLevel::VeryActive  => 1.90,
            ActivityLevel::ExtraActive => 1.95,
        }
    }
}

/// Calculate total daily energy expenditure (TDEE) from BMR and activity level
///
/// # Arguments
/// - `bmr`: Basal metabolic rate in calories/day
/// - `activity_level`: Activity level enum
///
/// # Returns
/// - TDEE in calories/day
pub fn calculate_tdee(bmr: f64, activity_level: ActivityLevel) -> f64 {
    bmr * activity_level.factor()
}
