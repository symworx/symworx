// core/statistics/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

// ==========================================================
// MODULES
// ==========================================================
pub mod autocorrelation;
pub mod basic;
pub mod correlation;
pub mod distance;
pub mod errors;
pub mod linreg;
pub mod pca;
pub mod py;
pub mod variability;

// ==========================================================
// EXPORTS
// ==========================================================
pub use autocorrelation::{
    acf
};
pub use basic::{
    mean,
    median,
    mad,
    percentile,
};
pub use correlation::{
    pearson_correlation, 
    correlation_matrix, 
    correlation_matrix_from_vec,
}; 
pub use distance::{
    euclidean,
};
pub use errors::{
    mae,
    mse,
    rmse,
};
pub use linreg::{
    l1,
    l2,
};
pub use variability::{
    intervals,
    ibi,
    rmssd,
    sdnn,
};

// ==========================================================
// PYTHON REGISTER
// ==========================================================
pub use py::{
    // --- Autocorrelation ---
    py_acf,
    // --- Basic statistics ---
    py_mean,
    py_median,
    py_mad,
    py_percentile,
    // --- Correlation ---
    py_pearson_correlation,
    py_correlation_matrix,
    py_correlation_matrix_from_vec,
    // --- Distance ---
    py_euclidean,
    // --- Errors ---
    py_mae,
    py_mse,
    py_rmse,
    // --- Linear regression ---
    py_l1,
    py_l2,
    // --- Variability ---
    py_intervals,
    py_ibi,
    py_rmssd,
    py_sdnn,
};

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {

    // --- Autocorrelation ----------------------------------
    m.add_function(wrap_pyfunction!(py_acf, m)?)?;

    // --- Basic statistics ---------------------------------
    m.add_function(wrap_pyfunction!(py_mean, m)?)?;
    m.add_function(wrap_pyfunction!(py_median, m)?)?;
    m.add_function(wrap_pyfunction!(py_mad, m)?)?;
    m.add_function(wrap_pyfunction!(py_percentile, m)?)?;

    // --- Correlation --------------------------------------
    let _ = m.add_function(wrap_pyfunction!(py_pearson_correlation, m)?);
    m.add_function(wrap_pyfunction!(py_correlation_matrix, m)?)?;
    let _ = m.add_function(wrap_pyfunction!(py_correlation_matrix_from_vec, m)?);

    // --- Distance -----------------------------------------
    m.add_function(wrap_pyfunction!(py_euclidean, m)?)?;

    // --- Errors -------------------------------------------
    m.add_function(wrap_pyfunction!(py_mae, m)?)?;
    m.add_function(wrap_pyfunction!(py_mse, m)?)?;
    m.add_function(wrap_pyfunction!(py_rmse, m)?)?;
 
    // --- Linear regression --------------------------------
    m.add_function(wrap_pyfunction!(py_l1, m)?)?;
    m.add_function(wrap_pyfunction!(py_l2, m)?)?;

    // --- Variability --------------------------------------
    m.add_function(wrap_pyfunction!(py_intervals, m)?)?;
    m.add_function(wrap_pyfunction!(py_ibi, m)?)?;
    m.add_function(wrap_pyfunction!(py_rmssd, m)?)?;
    m.add_function(wrap_pyfunction!(py_sdnn, m)?)?;

    Ok(())
}
