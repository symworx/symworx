// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Calorie and energy expenditure calculations: BMR (Mifflin-St Jeor + obesity
//! adjustments), TDEE, BMI, deficit targets, and calorie intake/activity splits.
//!
//! See the parent [`crate::nutrition`] module for overview and unit conventions
//! (height in meters, mass in kg, energy in kcal).

// Enums

/// Physical activity level used to estimate Total Daily Energy Expenditure (TDEE).
///
/// These multipliers are applied to BMR using the standard Harris-Benedict / Mifflin-St Jeor approach.
/// Typical real-world values range from ~1.2 (desk job, little exercise) to ~1.9+ (very hard training + physical job).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityLevel {
    /// Little or no exercise, desk job (×1.2)
    Sedentary,
    /// Light exercise or sports 1–3 days per week (×1.375)
    Light,
    /// Moderate exercise or sports 3–5 days per week (×1.55)
    Moderate,
    /// Hard exercise or sports 6–7 days per week (×1.725)
    Active,
    /// Very hard daily exercise or physical job (×1.9)
    VeryActive,
    /// Extremely hard training + physical job or 2× daily training (×1.95)
    ExtraActive,
}

impl ActivityLevel {
    /// Returns the activity multiplier used to compute TDEE from BMR.
    pub fn factor(self) -> f64 {
        match self {
            ActivityLevel::Sedentary => 1.20,
            ActivityLevel::Light => 1.375,
            ActivityLevel::Moderate => 1.55,
            ActivityLevel::Active => 1.725,
            ActivityLevel::VeryActive => 1.90,
            ActivityLevel::ExtraActive => 1.95,
        }
    }
}

/// Severity of caloric deficit used for weight loss target calculations.
///
/// Two equivalent ways of expressing the same target deficit are provided:
/// - A fixed daily calorie amount (`as_calories`)
/// - A percentage of active calories (`as_percent_of_active`)
///
/// The percentage approach (relative to `TDEE - BMR`) tends to scale better across
/// individuals of different sizes and activity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeficitLevel {
    /// Very conservative deficit (~150 kcal or 15% of active calories)
    Light,
    /// Mild, sustainable deficit (~300 kcal or 23% of active calories)
    Mild,
    /// Moderate, commonly recommended deficit (~600 kcal or 33% of active calories)
    Moderate,
    /// Aggressive deficit (~850 kcal or 45% of active calories)
    Aggressive,
    /// Very aggressive / rapid weight loss (~1000 kcal or 55% of active calories)
    Extreme,
}

impl DeficitLevel {
    /// Returns a fixed daily caloric deficit in kcal.
    ///
    /// These values are capped internally by [`calculate_deficit`] so that
    /// intake never drops below BMR.
    pub fn as_calories(self) -> f64 {
        match self {
            DeficitLevel::Light => 150.0,
            DeficitLevel::Mild => 300.0,
            DeficitLevel::Moderate => 600.0,
            DeficitLevel::Aggressive => 850.0,
            DeficitLevel::Extreme => 1000.0,
        }
    }

    /// Returns the deficit expressed as a fraction of active calories (`TDEE - BMR`).
    ///
    /// This form is used by [`calculate_deficit_from_active`].
    pub fn as_percent_of_active(self) -> f64 {
        match self {
            DeficitLevel::Light => 0.15,
            DeficitLevel::Mild => 0.23,
            DeficitLevel::Moderate => 0.33,
            DeficitLevel::Aggressive => 0.45,
            DeficitLevel::Extreme => 0.55,
        }
    }
}

/// Strategy for distributing a daily caloric deficit between reduced food intake and increased activity.
///
/// This affects the split between:
/// - Lower target caloric intake, and
/// - Higher target activity calories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeficitStrategy {
    /// 75% of the deficit comes from eating less, 25% from moving more.
    CaloricRestriction,
    /// 25% of the deficit comes from eating less, 75% from moving more.
    ActivityIncrease,
    /// Even 50/50 split between intake reduction and activity increase.
    Balanced,
}

impl DeficitStrategy {
    /// Returns the `(intake_portion, activity_portion)` split for this strategy.
    ///
    /// The two values always sum to 1.0.
    pub fn split(self) -> (f64, f64) {
        match self {
            DeficitStrategy::CaloricRestriction => (0.75, 0.25),
            DeficitStrategy::ActivityIncrease => (0.25, 0.75),
            DeficitStrategy::Balanced => (0.5, 0.5),
        }
    }
}

/// Gender used to select the constant term in the Mifflin-St Jeor BMR equation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gender {
    /// Male (constant +5)
    Male,
    /// Female (constant -161)
    Female,
}

/// Strategy for adjusting BMR calculations when BMI indicates obesity.
///
/// Obesity can cause overestimation of BMR when using raw weight (much of the
/// mass is not metabolically active). Two common adjustment approaches are provided.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObesityAdjustment {
    /// No adjustment (use raw weight in BMR equation).
    None,
    /// Use adjusted body weight = ideal_weight + factor * (actual - ideal).
    ///
    /// Common clinical choice, factor=0.25 means 25% of excess weight is counted.
    /// Ideal weight here is based on BMI=22.5.
    AdjustedWeight { factor: f64 },
    /// For BMI > 30, smoothly reduce the weight coefficient (10.0) in MSJ.
    ///
    /// Quadratic reduction up to a cap; provides a continuous transition without
    /// an explicit "ideal weight".
    ReducedCoefficient,
}

/// Configuration for BMR calculation, primarily controlling obesity adjustment behavior.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BmrConfig {
    pub obesity_adjustment: ObesityAdjustment,
}

impl Default for BmrConfig {
    fn default() -> Self {
        Self {
            obesity_adjustment: ObesityAdjustment::AdjustedWeight { factor: 0.25 },
        }
    }
}

// Functions

/// Calculate Basal Metabolic Rate (BMR) using the Mifflin-St Jeor equation.
///
/// Returns estimated daily energy expenditure in **kcal** while at complete rest.
///
/// This is the most widely used modern BMR equation for adults.
///
/// # Arguments
/// - `height_m`: Height in **meters** (SymWorx convention). The function converts
///   internally to cm because the Mifflin-St Jeor equation is traditionally defined with cm.
/// - `config`: Controls obesity adjustment (see [`BmrConfig`] and [`ObesityAdjustment`]).
///
/// Returns `f64::NAN` for clearly invalid inputs (age outside adult range, unrealistic
/// height/weight). Callers should handle NaN or pre-validate.
pub fn calculate_bmr(weight_kg: f64, height_m: f64, age_years: f64, gender: Gender, config: BmrConfig) -> f64 {
    // Basic validation (adult-oriented; formula not intended for children/elderly extremes)
    if weight_kg < 20.0 || height_m < 0.5 || !(18.0..=99.0).contains(&age_years) {
        return f64::NAN;
    }

    let height_cm = height_m * 100.0;
    let bmi = calculate_bmi(weight_kg, height_m);

    // Baseline Mifflin-St Jeor (using actual weight)
    let mut bmr = match gender {
        Gender::Male => 10.0 * weight_kg + 6.25 * height_cm - 5.0 * age_years + 5.0,
        Gender::Female => 10.0 * weight_kg + 6.25 * height_cm - 5.0 * age_years - 161.0,
    };

    // Apply obesity adjustment if configured and applicable
    if let Some(adjusted_weight) = get_adjusted_weight(weight_kg, height_m, bmi, config.obesity_adjustment) {
        bmr = match gender {
            Gender::Male => 10.0 * adjusted_weight + 6.25 * height_cm - 5.0 * age_years + 5.0,
            Gender::Female => 10.0 * adjusted_weight + 6.25 * height_cm - 5.0 * age_years - 161.0,
        };
    } else if let ObesityAdjustment::ReducedCoefficient = config.obesity_adjustment
        && bmi > 30.0
    {
        let excess_bmi = bmi - 30.0;
        let reduction = (0.018 * excess_bmi * excess_bmi).min(4.5);
        let weight_coeff = (10.0 - reduction).max(6.0);

        bmr = weight_coeff * weight_kg + 6.25 * height_cm - 5.0 * age_years
            + if gender == Gender::Male { 5.0 } else { -161.0 };
    }

    bmr.round()
}

/// Returns adjusted weight if applicable, otherwise None
fn get_adjusted_weight(weight_kg: f64, height_m: f64, bmi: f64, adjustment: ObesityAdjustment) -> Option<f64> {
    match adjustment {
        ObesityAdjustment::AdjustedWeight { factor } if bmi > 30.0 => {
            let ideal_weight = 22.5 * height_m * height_m; // Healthy BMI midpoint
            Some(ideal_weight + factor * (weight_kg - ideal_weight))
        }
        _ => None,
    }
}

/// Calculate Total Daily Energy Expenditure (TDEE) from BMR and activity level.
///
/// This is simply `bmr * activity_level.factor()`.
pub fn calculate_tdee(bmr: f64, activity_level: ActivityLevel) -> f64 {
    bmr * activity_level.factor()
}

/// Calculate Body Mass Index (BMI).
///
/// Formula: `BMI = weight_kg / (height_m * height_m)`
///
/// # Arguments
/// - `height_m`: meters (SymWorx convention).
///
/// Returns the standard BMI value (kg/m²). Returns NaN if height <= 0.
pub fn calculate_bmi(weight_kg: f64, height_m: f64) -> f64 {
    if height_m <= 0.0 {
        return f64::NAN;
    }
    weight_kg / (height_m * height_m)
}

/// Calculate a target daily caloric deficit using fixed amounts.
///
/// The returned deficit is capped so that daily intake never falls below BMR
/// (i.e. the maximum possible deficit is `tdee - bmr`).
pub fn calculate_deficit(bmr: f64, tdee: f64, deficit_level: DeficitLevel) -> f64 {
    let max_deficit = (tdee - bmr).max(0.0);
    deficit_level.as_calories().min(max_deficit)
}

/// Calculate a target daily caloric deficit as a percentage of active calories.
///
/// Active calories = `tdee - bmr`.
///
/// This approach scales the deficit relative to the individual's size and activity level.
pub fn calculate_deficit_from_active(bmr: f64, tdee: f64, deficit_level: DeficitLevel) -> f64 {
    let active_calories = tdee - bmr;
    active_calories * deficit_level.as_percent_of_active()
}

/// Given a total daily deficit and a strategy, compute the corresponding
/// daily caloric intake target and activity calorie target.
///
/// Returns `(target_intake_kcal, target_activity_kcal)`.
///
/// The intake target is guaranteed to be at least `bmr`.
pub fn calculate_calorie_targets(tdee: f64, bmr: f64, deficit: f64, strategy: DeficitStrategy) -> (f64, f64) {
    let (calorie_portion, activity_portion) = strategy.split();

    let deficit_from_calories = deficit * calorie_portion;
    let deficit_from_activity = deficit * activity_portion;

    let target_intake = (tdee - deficit_from_calories).max(bmr);
    let target_activity = (tdee - bmr) + deficit_from_activity;

    (target_intake, target_activity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_bmi_standard() {
        let bmi = calculate_bmi(70.0, 1.75);
        assert!((bmi - 22.857).abs() < 0.01);
    }

    #[test]
    fn test_calculate_bmi_zero_height() {
        let bmi = calculate_bmi(70.0, 0.0);
        assert!(bmi.is_nan());
    }

    #[test]
    fn test_calculate_bmr_mifflin_male() {
        // 30yo male, 70kg, 1.75m — baseline (no obesity)
        let bmr = calculate_bmr(70.0, 1.75, 30.0, Gender::Male, BmrConfig::default());
        assert!(bmr > 1600.0 && bmr < 1700.0);
    }

    #[test]
    fn test_calculate_bmr_with_obesity_adjustment() {
        // Obese male: 120kg, 1.70m (~41.5 BMI)
        let weight = 120.0;
        let height = 1.70;
        let age = 35.0;
        let baseline = calculate_bmr(
            weight,
            height,
            age,
            Gender::Male,
            BmrConfig {
                obesity_adjustment: ObesityAdjustment::None,
            },
        );
        let adjusted = calculate_bmr(weight, height, age, Gender::Male, BmrConfig::default()); // default = AdjustedWeight 0.25
        // Adjusted should be noticeably lower than raw (less weight plugged into coeff)
        assert!(adjusted < baseline - 50.0);
        assert!(adjusted.is_finite());
    }

    #[test]
    fn test_calculate_calorie_targets() {
        let (intake, activity) = calculate_calorie_targets(2500.0, 1600.0, 500.0, DeficitStrategy::Balanced);
        assert!((intake - 2250.0).abs() < 1.0);
        // target activity = active cals + portion of deficit
        assert!(activity > 1100.0 && activity < 1200.0);
    }
}
