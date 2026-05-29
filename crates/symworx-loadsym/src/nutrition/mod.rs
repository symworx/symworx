// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

pub mod bodycomp;

pub use bodycomp::{
    ActivityLevel,
    DeficitLevel,
    DeficitStrategy, 
    WeightlossModel,
    calculate_bmr,
    calculate_tdee,
    calculate_deficit,
    calculate_deficit_from_active,
    calculate_weightloss, 
};
