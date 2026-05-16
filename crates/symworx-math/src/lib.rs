// core/src/math/mod.rs
// Copyright (C) 2026 cSYMd

pub mod distributions;
pub mod integration;
pub mod random;
pub mod special;

pub use distributions::{
    beta_kernel,
    beta_pdf,
    gamma_kernel,
    gamma_pdf,
};

pub use integration::{
    cumtrapz,
    trapz,
};

pub use special::{
    gamma,
    ln_gamma,
    beta,
    ln_beta,
};

pub use random::*;

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
