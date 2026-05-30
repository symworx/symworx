// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Body composition and energy balance modeling.
//!
//! This module provides tools for estimating daily energy needs and
//! simulating weight loss trajectories. It is commonly used alongside
//! training load calculations in `symworx-loadsym`.
//!
//! # Key Concepts
//!
//! - **BMR** (Basal Metabolic Rate): Energy expended at complete rest.
//! - **TDEE** (Total Daily Energy Expenditure): BMR scaled by activity level.
//! - **Caloric Deficit**: The daily energy shortfall used to drive weight loss.
//! - **Deficit Strategy**: How a deficit is split between reduced intake and
//!   increased activity.
//!
//! All energy values are in **kilocalories (kcal)**.

const KCAL_PER_KG: f64 = 7700.0;

// Enums

/// Physical activity level used to estimate Total Daily Energy Expenditure (TDEE).
///
/// These multipliers are applied to BMR using the standard Harris-Benedict / Mifflin-St Jeor approach.
/// Typical real-world values range from ~1.2 (desk job, little exercise) to ~1.9+ (very hard training + physical job).
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy)]
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

// Structs

/// Result of a weight-loss simulation produced by [`calculate_weightloss`].
///
/// Each vector has the same length and corresponds to one simulated week.
/// The first entry (`index 0`) represents the starting state (week 0).
#[derive(Debug, Clone)]
pub struct WeightlossModel {
    /// The deficit severity used for the simulation.
    pub deficit_level: DeficitLevel,
    /// How the deficit was split between intake and activity.
    pub deficit_strategy: DeficitStrategy,

    /// Week number (0 = start, 1 = after first week of deficit, ...).
    pub week: Vec<u32>,
    /// Body weight in kilograms at the end of each week.
    pub weight_kg: Vec<f64>,
    /// Body Mass Index (BMI) at the end of each week.
    pub bmi: Vec<f64>,
    /// Caloric deficit applied during that week.
    pub weekly_deficit_kcal: Vec<f64>,
    /// Running total of all deficits accumulated so far.
    pub total_deficit_kcal: Vec<f64>,
}

// Functions 

/// Calculate Basal Metabolic Rate (BMR) using the Mifflin-St Jeor equation.
///
/// Returns estimated daily energy expenditure in **kcal** while at complete rest.
///
/// This is the most widely used modern BMR equation for adults.
///
/// # Arguments
/// - `height_m`: Height in **meters**. The function converts internally to cm
///   because the Mifflin-St Jeor equation is traditionally defined with cm.
pub fn calculate_bmr(weight_kg: f64, height_m: f64, age_years: f64, is_male: bool) -> f64 {
    let height_cm = height_m * 100.0;
    if is_male {
        10.0 * weight_kg + 6.25 * height_cm - 5.0 * age_years + 5.0
    } else {
        10.0 * weight_kg + 6.25 * height_cm - 5.0 * age_years - 161.0
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
/// Returns the standard BMI value (kg/m²).
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
    let mut deficit = deficit_level.as_calories();

    if (tdee - deficit) > bmr {
        deficit = tdee - bmr;
    }

    deficit
}

/// Calculate a target daily caloric deficit as a percentage of active calories.
///
/// Active calories = `tdee - bmr`.
///
/// This approach scales the deficit relative to the individual's size and activity level.
pub fn calculate_deficit_from_active(bmr: f64, tdee: f64, deficit_level: DeficitLevel) -> f64 {
    let active_calories = tdee - bmr;
    let deficit = active_calories * deficit_level.as_percent_of_active();

    deficit
}

/// Given a total daily deficit and a strategy, compute the corresponding
/// daily caloric intake target and activity calorie target.
///
/// Returns `(target_intake_kcal, target_activity_kcal)`.
///
/// The intake target is guaranteed to be at least `bmr`.
pub fn calculate_calorie_targets(
    tdee: f64,
    bmr: f64,
    deficit: f64,
    strategy: DeficitStrategy,
) -> (f64, f64) {
    let (calorie_portion, activity_portion) = strategy.split();

    let deficit_from_calories = deficit * calorie_portion;
    let deficit_from_activity = deficit * activity_portion;

    let target_intake = (tdee - deficit_from_calories).max(bmr);
    let target_activity = (tdee - bmr) + deficit_from_activity;

    (target_intake, target_activity)
}


/// Simulate a weekly weight-loss trajectory from a starting weight to a target weight.
///
/// The simulation uses a constant weekly deficit based on the provided
/// [`DeficitLevel`] and [`DeficitStrategy`]. Weight loss is modeled at
/// approximately **7700 kcal per kg** of body fat.
///
/// # Arguments
/// - `starting_weight_kg`, `target_weight_kg`: Current and goal weights.
/// - `activity_level`: Used to estimate TDEE at each step (recalculated as weight drops).
/// - `deficit_level` + `strategy`: Control how aggressive the deficit is and how it is split.
///
/// # Note on units
/// `height_m` is expected in **meters** (the SymWorx workspace convention for body measurements).
///
/// # Returns
/// A [`WeightlossModel`] containing parallel vectors for each simulated week.
///
/// # Safety & Termination
/// The simulation stops when any of the following occur:
/// - Target weight is reached (within 50 g).
/// - BMI drops below 18.0 (clinically underweight / unsafe zone).
/// - More than 78 weeks (~1.5 years) have elapsed.
///
/// The returned trajectory is always safe to use for display or further analysis,
/// but callers should inspect the final BMI and total duration.
pub fn calculate_weightloss(
    age_years: f64,
    is_male: bool,
    height_m: f64,
    starting_weight_kg: f64,
    target_weight_kg: f64,
    activity_level: ActivityLevel,
    deficit_level: DeficitLevel,
    strategy: DeficitStrategy,
) -> WeightlossModel {
    let mut trajectory = WeightlossModel {
        deficit_level,
        deficit_strategy: strategy,
        week: vec![],
        weight_kg: vec![],
        bmi: vec![],
        weekly_deficit_kcal: vec![],
        total_deficit_kcal: vec![],
    };

    let mut current_weight = starting_weight_kg;
    let mut week = 0u32;
    let mut cumulative_deficit = 0.0;

    while current_weight > target_weight_kg + 0.05 {
        // Calculate current BMI using the dedicated helper (expects meters)
        let bmi = calculate_bmi(current_weight, height_m);

        // Calculate current BMR and TDEE (BMR now expects meters)
        let bmr = calculate_bmr(current_weight, height_m, age_years, is_male);
        let tdee = calculate_tdee(bmr, activity_level);

        // Calculate this week's deficit
        let weekly_deficit = calculate_deficit_from_active(tdee, bmr, deficit_level);

        // Apply weight loss (1 kg ≈ 7700 kcal)
        let weight_loss_this_week = weekly_deficit / KCAL_PER_KG;
        current_weight -= weight_loss_this_week;

        // Prevent overshooting the target
        if current_weight < target_weight_kg {
            current_weight = target_weight_kg;
        }

        // Record data
        cumulative_deficit += weekly_deficit;

        trajectory.week.push(week);
        trajectory.weight_kg.push(current_weight);
        trajectory.bmi.push(bmi);
        trajectory.weekly_deficit_kcal.push(weekly_deficit);
        trajectory.total_deficit_kcal.push(cumulative_deficit);

        week += 1;

        // Safety measures
        if bmi < 18.0 {
            // unsafe / unrecommended BMI
            break;
        }
        if week > 78 {
            // 1.5 years
            break;
        }
    }

    trajectory
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
        // 30yo male, 70kg, 1.75m
        let bmr = calculate_bmr(70.0, 1.75, 30.0, true);
        assert!(bmr > 1600.0 && bmr < 1700.0);
    }

    #[test]
    fn test_calculate_calorie_targets() {
        let (intake, activity) = calculate_calorie_targets(2500.0, 1600.0, 500.0, DeficitStrategy::Balanced);
        assert!((intake - 2250.0).abs() < 1.0);
        // target activity = active cals + portion of deficit
        assert!(activity > 1100.0 && activity < 1200.0);
    }

    #[test]
    fn test_calculate_weightloss_smoke() {
        // Just ensure the simulation runs and produces a trajectory without crashing
        let model = calculate_weightloss(
            30.0,
            true,
            1.75,
            82.0,
            78.0,
            ActivityLevel::Moderate,
            DeficitLevel::Moderate,
            DeficitStrategy::Balanced,
        );
        assert!(!model.week.is_empty());
        assert_eq!(model.weight_kg.len(), model.week.len());
        // BMI should be plausible throughout
        for &b in &model.bmi {
            assert!(b > 15.0 && b < 35.0);
        }
    }
}
