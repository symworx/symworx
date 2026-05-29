// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

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

// Public API
/// Autocorrelation functions.
pub mod autocorrelation;

/// Basic statistics, including mean, median, mad.
pub mod basic;

/// Correlation functions.
pub mod correlation;

/// Distance metrics (e.g., Euclidean).
pub mod distance;

/// Errors measurements.
pub mod error_metrics;

/// Linear regression models (e.g., l1 and l2).
pub mod linreg;

/// Principal component analysis.
pub mod pca;

/// Singular value decomposition.
pub mod svd;

/// Variability measurements (e.g., ibi, rmssd, sdnn)
pub mod variability;

// Re-exports
pub use autocorrelation::acf;
pub use basic::{mad, mean, median, percentile};
pub use correlation::{correlation_matrix, correlation_matrix_from_vec, pearson_correlation};
pub use distance::{chebyshev, cosine_distance, euclidean, manhattan};
pub use error_metrics::{mae, mse, rmse};
pub use linreg::{l1, l2};
pub use variability::{
    mean_successive_differences, rmssd, sd_successive_differences, successive_differences,
};

// Version info
/// Current version of the `symworx-stats` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
