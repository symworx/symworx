// core/src/math/mod.rs
// Copyright (C) 2026 cSYMd

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

// ==========================================================
// Modules
// ==========================================================

pub mod distributions;
pub mod integration;
pub mod random;
pub mod special;
pub mod py;

// ==========================================================
// Namespaced re-exports
// ==========================================================

/// Probability distributions: kernels, PDFs, sampling.
pub mod dist {
    pub use super::distributions::*;
}

/// Special mathematical functions (Gamma, Beta, etc.).
pub mod special_fn {
    pub use super::special::*;
}

/// Numerical integration methods.
pub mod integrate {
    pub use super::integration::*;
}

// /// Random sampling utilities.
// pub mod random {
//     pub use super::random::*;
// }

// ==========================================================
// Top-level exports
// ==========================================================

// Distributions
pub use distributions::{
    beta_kernel,
    beta_pdf,
    gamma_kernel,
    gamma_pdf,
};

// Integration
pub use integration::{
    cumtrapz,
    trapz,
};

// Special functions
pub use special::{
    gamma,
    ln_gamma,
    beta,
    ln_beta,
};

// Random sampling
pub use random::*;

// ==========================================================
// Python Registration
// ==========================================================

pub use py::{
    // Distributions
    py_beta_kernel,
    py_beta_pdf,
    py_beta_sample,
    py_gamma_kernel,
    py_gamma_pdf,
    py_gamma_sample,
    py_normal_sample,
    // Integration
    py_cumtrapz,
    py_trapz, 
};

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Distributions
    m.add_function(wrap_pyfunction!(py_beta_kernel, m)?)?;
    m.add_function(wrap_pyfunction!(py_beta_pdf, m)?)?;
    m.add_function(wrap_pyfunction!(py_gamma_kernel, m)?)?;
    m.add_function(wrap_pyfunction!(py_gamma_pdf, m)?)?;

    // Integration
    m.add_function(wrap_pyfunction!(py_cumtrapz, m)?)?;
    m.add_function(wrap_pyfunction!(py_trapz, m)?)?;

    // Sample 
    m.add_function(wrap_pyfunction!(py_beta_sample, m)?)?;
    m.add_function(wrap_pyfunction!(py_gamma_sample, m)?)?;
    m.add_function(wrap_pyfunction!(py_normal_sample, m)?)?;
    Ok(())
}
