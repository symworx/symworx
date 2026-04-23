// core/statistics/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

pub mod autocorrelation;
pub mod basic;
pub mod correlation;
pub mod linreg;
pub mod pca;
pub mod py;

// re-exports of specific statistics types
pub use autocorrelation::{acf};
pub use basic::{mean, median};
pub use correlation::{
    pearson_correlation, 
    correlation_matrix, 
    correlation_matrix_from_vec
}; 
pub use linreg::{l1, l2};

pub use py::{
    py_acf,
    py_mean,
    py_median,
    py_pearson_correlation,
    py_correlation_matrix,
    py_correlation_matrix_from_vec,
    py_l1,
    py_l2
};

// ==========================================================
// PYTHON REGISTER
// ==========================================================
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {

    // --- autocorrelation ---
    m.add_function(wrap_pyfunction!(py_acf, m)?)?;

    // --- basic statistics ---
    m.add_function(wrap_pyfunction!(py_mean, m)?)?;
    m.add_function(wrap_pyfunction!(py_median, m)?)?;

    // ---  ---
    m.add_function(wrap_pyfunction!(py_l1, m)?)?;
    m.add_function(wrap_pyfunction!(py_l2, m)?)?;

    Ok(())
}
