// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

/// Placeholder physiological load: mean of HR samples (not a real load metric).
pub fn calculate_physiological_load(hr_data: &[f64]) -> f64 {
    // Placeholder — mean HR only; replace with a real load model.
    hr_data.iter().sum::<f64>() / hr_data.len() as f64
}
