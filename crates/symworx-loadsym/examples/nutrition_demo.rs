// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Demonstration of the nutrition / body composition module.
//!
//! Run with:
//!   cargo run -p symworx-loadsym --example nutrition_demo

use symworx_loadsym::nutrition::{
    ActivityLevel,
    BmrConfig,
    DeficitLevel,
    DeficitStrategy,
    Gender,
    calculate_bmi,
    calculate_bmr,
    calculate_calorie_targets,
    calculate_tdee,
    calculate_weightloss,
};

fn main() {
    println!("=== SymWorx LoadSym Nutrition Demo ===\n");

    let weight = 78.5;
    let height_m = 1.78;
    let age = 32.0;
    let is_male = true;

    let gender = if is_male {
        Gender::Male
    } else {
        Gender::Female
    };
    let bmr = calculate_bmr(weight, height_m, age, gender, BmrConfig::default());
    let bmi = calculate_bmi(weight, height_m);
    let tdee = calculate_tdee(bmr, ActivityLevel::Moderate);

    println!("Athlete profile:");
    println!("  Weight : {:.1} kg", weight);
    println!("  Height : {:.2} m (BMI = {:.1})", height_m, bmi);
    println!(
        "  Age    : {:.0} years ({})",
        age,
        if is_male { "male" } else { "female" }
    );
    println!("  BMR    : {:.0} kcal", bmr);
    println!("  TDEE (moderate) : {:.0} kcal\n", tdee);

    let (intake, activity) = calculate_calorie_targets(tdee, bmr, 550.0, DeficitStrategy::Balanced);
    println!("Target for ~550 kcal daily deficit (balanced strategy):");
    println!("  Daily intake target : {:.0} kcal", intake);
    println!("  Daily activity target : {:.0} kcal\n", activity);

    println!("Running 12-week weight loss simulation (moderate deficit)...");
    let trajectory = calculate_weightloss(
        age,
        gender,
        height_m,
        weight,
        72.0, // target
        ActivityLevel::Moderate,
        DeficitLevel::Moderate,
        DeficitStrategy::Balanced,
        BmrConfig::default(),
    );

    let weeks = trajectory.week.len();
    let final_w = *trajectory.weight_kg.last().unwrap();
    let final_bmi = *trajectory.bmi.last().unwrap();

    println!("Simulation finished after {} weeks", weeks);
    println!("  Final weight : {:.1} kg", final_w);
    println!("  Final BMI    : {:.1}", final_bmi);
    println!("\nDemo complete.");
}
