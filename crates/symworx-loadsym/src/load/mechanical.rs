// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

/// Calculate mechanical load from force and velocity data
pub fn calculate_mechanical_load(force_data: &[f64], velocity_data: &[f64]) -> f64 {
    force_data
        .iter()
        .zip(velocity_data.iter())
        .map(|(f, v)| f * v)
        .sum::<f64>()
        / force_data.len() as f64
}
