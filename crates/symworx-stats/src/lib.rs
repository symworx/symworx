// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! # symworx-stats
//!
//! Statistical analysis tools for SymWorx.
//!
//! Focused on methods commonly used in physiological signal analysis,
//! biomechanics, and training load research (variability, correlation,
//! regression, entropy, etc.).
//!
//! ## Linear algebra features
//!
//! SVD, PCA, and the high-precision `l2` (OLS) / `l1` (Lasso) regression
//! functions require the **`linalg`** feature on `symworx-stats`:
//!
//! ```toml
//! [dependencies]
//! symworx-stats = { version = "...", features = ["linalg"] }
//! ```
//!
//! This feature pulls `ndarray-linalg` (and transitively `cauchy` + a LAPACK
//! backend such as OpenBLAS).
//!
//! **It is off by default** in the standalone `symworx-stats` crate to keep the
//! dependency footprint minimal for basic statistics use cases.
//!
//! However, `symworx-core` enables the `linalg` feature by default for
//! convenience, so most users of the ecosystem get regression, SVD, and PCA
//! without extra configuration.

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
///
/// The high-quality `l2` implementation (and therefore `l1` which can call it)
/// requires the `linalg` feature (which brings ndarray-linalg + cauchy etc.).
pub mod linreg;

#[cfg(feature = "linalg")]
/// Principal component analysis (requires `linalg` feature).
pub mod pca;

#[cfg(feature = "linalg")]
/// Singular value decomposition (requires `linalg` feature).
pub mod svd;

/// Variability measurements (e.g., ibi, rmssd, sdnn)
pub mod variability;

// Re-exports
pub use autocorrelation::acf;
pub use basic::{cv, mad, mean, median, percentile, std_dev, std_dev_sample};
pub use correlation::{correlation_matrix, correlation_matrix_from_vec, pearson_correlation};
pub use distance::{chebyshev, cosine_distance, euclidean, manhattan};
pub use error_metrics::{mae, mse, rmse};
#[cfg(feature = "linalg")]
pub use linreg::{l1, l2};
pub use variability::{
    mean_successive_differences, rmssd, sd_successive_differences, successive_differences,
};

// Version info
/// Current version of the `symworx-stats` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
