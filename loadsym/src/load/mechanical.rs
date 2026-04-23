// loadsym/src/load/mechanical.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;

/// Calculate mechanical load from force and velocity data
pub fn calculate_mechanical_load(force_data: &[f64], velocity_data: &[f64]) -> f64 {
    force_data.iter().zip(velocity_data.iter())
        .map(|(f, v)| f * v)
        .sum::<f64>() / force_data.len() as f64
}

#[pyfunction(
    name = "calculate_mechanical_load",
    signature = (force_data, velocity_data),
)]
pub fn py_calculate_mechanical_load(force_data: Vec<f64>, velocity_data: Vec<f64>) -> f64 {
    if force_data.len() != velocity_data.len() {
        panic!("force_data and velocity_data must have the same length ({} vs {})", force_data.len(), velocity_data.len());
    }

    calculate_mechanical_load(&force_data, &velocity_data)
}
