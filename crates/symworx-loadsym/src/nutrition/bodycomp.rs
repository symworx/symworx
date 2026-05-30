// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

const KCAL_PER_KG: f64 = 7700.0;

// Enums
/// Activity level enum for TDEE.
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
            ActivityLevel::Sedentary => 1.20,
            ActivityLevel::Light => 1.375,
            ActivityLevel::Moderate => 1.55,
            ActivityLevel::Active => 1.725,
            ActivityLevel::VeryActive => 1.90,
            ActivityLevel::ExtraActive => 1.95,
        }
    }
}

/// Deficit level enum for weightloss target calculations.
#[derive(Debug, Clone, Copy)]
pub enum DeficitLevel {
    Light,
    Mild,
    Moderate,
    Aggressive,
    Extreme,
}

impl DeficitLevel {
    pub fn as_calories(self) -> f64 {
        match self {
            DeficitLevel::Light => 150.0,
            DeficitLevel::Mild => 300.0,
            DeficitLevel::Moderate => 600.0,
            DeficitLevel::Aggressive => 850.0,
            DeficitLevel::Extreme => 1000.0,
        }
    }
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

/// Deficit balance enum to split deficit across intake restriction and activity.
#[derive(Debug, Clone, Copy)]
pub enum DeficitStrategy {
    /// Primarily caloric restriction.
    CaloricRestriction,
    /// Primarily increases in activity.
    ActivityIncrease,
    /// Balanced
    Balanced,
}
impl DeficitStrategy {
    pub fn split(self) -> (f64, f64) {
        match self {
            DeficitStrategy::CaloricRestriction => (0.75, 0.25),
            DeficitStrategy::ActivityIncrease => (0.25, 0.75),
            DeficitStrategy::Balanced => (0.5, 0.5),
        }
    }
}

// Structs
/// Stores the full weight loss trajectory over time.
#[derive(Debug, Clone)]
pub struct WeightlossModel {
    pub deficit_level: DeficitLevel,
    pub deficit_strategy: DeficitStrategy,
    pub week: Vec<u32>,
    pub weight_kg: Vec<f64>,
    pub bmi: Vec<f64>,
    pub weekly_deficit_kcal: Vec<f64>,
    pub total_deficit_kcal: Vec<f64>,
}

// Functions 
/// Calculate basal metabolic rate (BMR) using the Mifflin-St Jeor Equation
pub fn calculate_bmr(weight_kg: f64, height_cm: f64, age_years: f64, is_male: bool) -> f64 {
    if is_male {
        10.0 * weight_kg + 6.25 * height_cm - 5.0 * age_years + 5.0
    } else {
        10.0 * weight_kg + 6.25 * height_cm - 5.0 * age_years - 161.0
    }
}

/// Calculate total daily energy expenditure (TDEE) from BMR and activity level
pub fn calculate_tdee(bmr: f64, activity_level: ActivityLevel) -> f64 {
    bmr * activity_level.factor()
}

/// calculate a target deficit using fixed caloric restriction targets.
pub fn calculate_deficit(bmr: f64, tdee: f64, deficit_level: DeficitLevel) -> f64 {
    let mut deficit = deficit_level.as_calories();

    if (tdee - deficit) > bmr {
        deficit = tdee - bmr;
    }

    deficit
}

/// Calculate a target deficit using a percentage of your acive calorie goal.
pub fn calculate_deficit_from_active(bmr: f64, tdee: f64, deficit_level: DeficitLevel) -> f64 {
    let active_calories = tdee - bmr;
    let deficit = active_calories * deficit_level.as_percent_of_active();

    deficit
}

/// Calculate target daily intake + activity calories
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


/// Simulate a weight loss journey from current weight to target weight.
pub fn calculate_weightloss(
    age_years: f64,
    is_male: bool,
    height_cm: f64,
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
        // Calculate current BMI
        let bmi = current_weight / (height_cm * height_cm);

        // Calculate current BMR and TDEE
        let bmr = calculate_bmr(current_weight, height_cm, age_years, is_male);
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
        if bmi < 18.0 { // unsafe, unrecommended BMI
            break;
        }
        if week > 78 { // 1.5 yrs
            break;
        }
    }

    trajectory
}
