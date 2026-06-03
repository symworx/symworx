// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Nutrition and body-composition modeling for SymWorx (BMR/TDEE, deficits,
//! weight-loss simulation).
//!
//! Submodules:
//! - [`calories`] — BMR (Mifflin-St Jeor + [`BmrConfig`]/obesity adjustments),
//!   TDEE, BMI, deficit levels/strategies, calorie target splits.
//! - [`weightloss`] — Weekly trajectory simulation producing self-describing
//!   [`weightloss::WeightlossModel`] (uses the above, re-exports main entrypoint).
//!
//! All public items are re-exported at this level for convenience.

pub mod calories;
pub mod weightloss;

// Re-export primary public API (calories + weightloss)
pub use calories::{
    ActivityLevel, BmrConfig, DeficitLevel, DeficitStrategy, Gender, ObesityAdjustment,
    calculate_bmi, calculate_bmr, calculate_calorie_targets, calculate_deficit,
    calculate_deficit_from_active, calculate_tdee,
};
pub use weightloss::{WeightlossModel, calculate_weightloss};
