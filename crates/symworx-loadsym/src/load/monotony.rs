// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Monotony and Strain (Foster's classic load monitoring metrics).
//!
//! Monotony = mean(daily_load) / sd(daily_load) over a week (or window).
//! Strain   = weekly (or window) total load × monotony.

use symworx_core::stats::{
    mean,
    std_dev,
};

use crate::error::{
    LoadSymError,
    Result,
};

/// Compute monotony over a window of daily loads.
///
/// Returns NaN (via error for now) on insufficient data.
pub fn compute_monotony(daily_loads: &[f64]) -> Result<f64> {
    if daily_loads.len() < 2 {
        return Err(LoadSymError::InsufficientData(
            "monotony requires at least 2 daily loads".into(),
        ));
    }
    let m = mean(daily_loads);
    let s = std_dev(daily_loads);
    if s == 0.0 || !s.is_finite() {
        // Perfectly consistent load -> very low monotony (or define as 1.0 by convention)
        return Ok(1.0);
    }
    Ok(m / s)
}

/// Compute strain = (sum of loads in window) × monotony.
pub fn compute_strain(daily_loads: &[f64]) -> Result<f64> {
    let mono = compute_monotony(daily_loads)?;
    let total: f64 = daily_loads.iter().sum();
    Ok(total * mono)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monotony_varied() {
        let loads = [300.0, 450.0, 200.0, 600.0, 350.0, 500.0, 280.0];
        let m = compute_monotony(&loads).unwrap();
        assert!(m > 1.0 && m < 3.0); // realistic for varied training week
    }

    #[test]
    fn test_monotony_constant() {
        let loads = [400.0; 7];
        let m = compute_monotony(&loads).unwrap();
        assert!((m - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_strain() {
        let loads = [300.0, 450.0, 200.0, 600.0, 350.0, 500.0, 280.0];
        let s = compute_strain(&loads).unwrap();
        assert!(s > 2000.0);
    }
}
