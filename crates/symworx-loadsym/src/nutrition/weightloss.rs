// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Weight loss trajectory simulation.
//!
//! Uses the calorie/BMR primitives from the sibling [`super::calories`] module
//! (BMR with optional obesity adjustment via [`BmrConfig`], TDEE, deficit
//! selection) to step a subject forward week-by-week, updating BMR as weight/BMI
//! changes.
//!
//! The primary export is [`calculate_weightloss`] producing a [`WeightlossModel`]

use super::calories::*;

const KCAL_PER_KG: f64 = 7700.0;

/// Result of a weight-loss simulation produced by [`calculate_weightloss`].
///
/// Each vector has the same length and corresponds to one simulated week.
/// The first entry (`index 0`) represents the starting state (week 0).
///
/// The model also records the full configuration used to drive BMR/TDEE
/// calculations at each step (including any [`BmrConfig`] obesity adjustment).
#[derive(Debug, Clone, PartialEq)]
pub struct WeightlossModel {
    // --- Configuration used for the entire simulation ---
    pub gender: Gender,
    pub bmr_config: BmrConfig,
    pub activity_level: ActivityLevel,
    pub age_years: f64,
    pub height_m: f64,
    pub starting_weight_kg: f64,
    pub target_weight_kg: f64,

    /// The deficit severity used for the simulation.
    pub deficit_level: DeficitLevel,
    /// How the deficit was split between intake and activity.
    pub deficit_strategy: DeficitStrategy,

    // --- Trajectory (parallel vectors, one entry per simulated week) ---
    /// Week number (0 = start / baseline, 1 = after week 1 of deficit, ...).
    pub week: Vec<u32>,
    /// Body weight in kilograms at the *end* of each week (week 0 = starting weight).
    pub weight_kg: Vec<f64>,
    /// Body Mass Index (BMI) at the end of each week.
    pub bmi: Vec<f64>,
    /// Caloric deficit applied *during that week* (weekly total = daily rate × 7; 0.0 for week 0).
    pub weekly_deficit_kcal: Vec<f64>,
    /// Running total of all deficits accumulated so far (in weekly-total units).
    pub total_deficit_kcal: Vec<f64>,
}

// Functions

/// Simulate a weekly weight-loss trajectory from a starting weight to a target weight.
///
/// At each weekly step the current BMR/TDEE (and thus deficit) is recomputed from the
/// current weight (and BMI, for any obesity adjustment in the `bmr_config`). Weight loss
/// is modeled at approximately **7700 kcal per kg** of body fat.
///
/// The `deficit_level` controls the *daily* target shortfall (either fixed kcal via
/// [`DeficitLevel::as_calories`] or relative via [`calculate_deficit_from_active`]).
/// The model integrates daily rate × 7 to obtain the weekly deficit total used for
/// the kg delta and for the `weekly_deficit_kcal` / `total_deficit_kcal` series.
///
/// # Arguments
/// - `starting_weight_kg`, `target_weight_kg`: Current and goal weights.
/// - `activity_level`: Used to estimate TDEE at each step (recalculated as weight drops).
/// - `deficit_level` + `strategy`: Control how aggressive the deficit is and how it is split.
/// - `bmr_config`: Obesity adjustment policy applied to every BMR evaluation in the trajectory.
///
/// # Note on units
/// `height_m` is expected in **meters** (the SymWorx workspace convention for body measurements).
///
/// # Returns
/// A [`WeightlossModel`] containing parallel vectors for each simulated week (index 0 = baseline).
/// The `weekly_*` and `total_*` deficit fields contain **weekly totals** (daily rate × 7).
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
    gender: Gender,
    height_m: f64,
    starting_weight_kg: f64,
    target_weight_kg: f64,
    activity_level: ActivityLevel,
    deficit_level: DeficitLevel,
    strategy: DeficitStrategy,
    bmr_config: BmrConfig,
) -> WeightlossModel {
    let mut trajectory = WeightlossModel {
        gender,
        bmr_config,
        activity_level,
        age_years,
        height_m,
        starting_weight_kg,
        target_weight_kg,
        deficit_level,
        deficit_strategy: strategy,
        week: vec![],
        weight_kg: vec![],
        bmi: vec![],
        weekly_deficit_kcal: vec![],
        total_deficit_kcal: vec![],
    };

    // Record starting state as week 0 (per struct docs)
    let init_bmi = calculate_bmi(starting_weight_kg, height_m);
    trajectory.week.push(0);
    trajectory.weight_kg.push(starting_weight_kg);
    trajectory.bmi.push(init_bmi);
    trajectory.weekly_deficit_kcal.push(0.0);
    trajectory.total_deficit_kcal.push(0.0);

    if starting_weight_kg <= target_weight_kg + 0.05 {
        return trajectory;
    }

    let mut current_weight = starting_weight_kg;
    let mut week: u32 = 0;
    let mut cumulative_deficit = 0.0;

    while current_weight > target_weight_kg + 0.05 {
        week += 1;

        // Pre-step values for BMR/TDEE (safety uses resulting post_bmi)
        let bmr = calculate_bmr(current_weight, height_m, age_years, gender, bmr_config);
        let tdee = calculate_tdee(bmr, activity_level);

        // Daily deficit (relative to active calories); the weight-loss model integrates to weekly total.
        let daily_deficit = calculate_deficit_from_active(bmr, tdee, deficit_level);
        let weekly_deficit = daily_deficit * 7.0; // total kcal for the modeled week

        // Apply weight loss (1 kg ≈ 7700 kcal). Use the *actual* planned deficit even on final partial week.
        let weight_loss_this_week = weekly_deficit / KCAL_PER_KG;
        current_weight -= weight_loss_this_week;

        // Prevent overshooting
        if current_weight < target_weight_kg {
            current_weight = target_weight_kg;
        }

        cumulative_deficit += weekly_deficit;

        let post_bmi = calculate_bmi(current_weight, height_m);

        trajectory.week.push(week);
        trajectory.weight_kg.push(current_weight);
        trajectory.bmi.push(post_bmi);
        trajectory.weekly_deficit_kcal.push(weekly_deficit);
        trajectory.total_deficit_kcal.push(cumulative_deficit);

        // Safety: stop if we entered or crossed the unsafe zone this week
        if post_bmi < 18.0 {
            break;
        }
        if week > 78 {
            break;
        }
    }

    trajectory
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_weightloss_smoke() {
        // Just ensure the simulation runs and produces a trajectory without crashing.
        // Now uses full config; model should expose the enums used.
        let model = calculate_weightloss(
            30.0,
            Gender::Male,
            1.75,
            82.0,
            78.0,
            ActivityLevel::Moderate,
            DeficitLevel::Moderate,
            DeficitStrategy::Balanced,
            BmrConfig::default(),
        );
        assert!(!model.week.is_empty());
        assert_eq!(model.weight_kg.len(), model.week.len());
        assert_eq!(model.week[0], 0);
        assert!((model.weight_kg[0] - 82.0).abs() < 0.01); // starting state recorded
        // BMI should be plausible throughout
        for &b in &model.bmi {
            assert!(b > 15.0 && b < 35.0);
        }
        // Model accounts for the enums
        assert_eq!(model.gender, Gender::Male);
        assert_eq!(model.activity_level, ActivityLevel::Moderate);
        assert_eq!(model.deficit_level, DeficitLevel::Moderate);
        assert!(model.bmr_config.obesity_adjustment != ObesityAdjustment::None); // default applies
    }

    #[test]
    fn test_weightloss_includes_start_and_config() {
        let cfg = BmrConfig {
            obesity_adjustment: ObesityAdjustment::None,
        };
        let model = calculate_weightloss(
            40.0,
            Gender::Female,
            1.65,
            95.0,
            70.0,
            ActivityLevel::Light,
            DeficitLevel::Light,
            DeficitStrategy::CaloricRestriction,
            cfg,
        );
        assert_eq!(model.week[0], 0);
        assert_eq!(model.gender, Gender::Female);
        assert_eq!(model.bmr_config, cfg);
        assert!(model.week.len() >= 2); // at least start + one step
    }
}
