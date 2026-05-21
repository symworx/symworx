// symworx/crates/symworx-stats/src/lib.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

//! # symworx-stats
//!
//! Statistical analysis tools for SymWorx.
//!
//! Focused on methods commonly used in physiological signal analysis,
//! biomechanics, and training load research (variability, correlation,
//! regression, entropy, etc.).

#![allow(unused_imports)]
#![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/symworx-stats")]

// ==========================================================
// Modules
// ==========================================================
pub mod autocorrelation;
pub mod basic;
pub mod correlation;
pub mod distance;
pub mod error;
pub mod linreg;
pub mod pca;
pub mod variability;

// ==========================================================
// Main re-exports
// ==========================================================
pub use autocorrelation::acf;
pub use basic::{
    mad, mean, median, percentile,
};
pub use correlation::{
    correlation_matrix, correlation_matrix_from_vec, pearson_correlation,
};
pub use distance::euclidean;
pub use error::{
    mae, mse, rmse,
};
pub use linreg::{l1, l2};
pub use variability::{
    ibi, intervals, rmssd, sdnn,
};

// ==========================================================
// Version info
// ==========================================================
/// Current version of the `symworx-stats` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

