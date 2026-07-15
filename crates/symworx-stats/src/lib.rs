// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

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
//! SVD, PCA, and the high-precision `l2`/`ols`/`ridge` regression solvers
//! require the **`linalg`** feature on `symworx-stats`:
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
//!
//! Lasso / Elastic Net (`l1`, `lasso`, `elastic_net`) and k-means clustering
//! do **not** require `linalg`.

#![allow(unused_imports)]
#![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/symworx-stats")]

// Public API
/// Autocorrelation functions.
pub mod autocorrelation;

/// Basic statistics, including mean, median, mad.
pub mod basic;

/// Clustering (k-means). Does not require the `linalg` feature.
pub mod cluster;

/// Correlation functions.
pub mod correlation;

/// Distance metrics (e.g., Euclidean).
pub mod distance;

/// Predicted-vs-expected error metrics (MAE, RMSE, R², regression report).
pub mod error_metrics;

/// Linear regression models (OLS, Ridge, Lasso, Elastic Net).
///
/// Closed-form `l2` / `ols` / `ridge` require the `linalg` feature.
/// Coordinate-descent `l1` / `lasso` / `elastic_net` do not.
pub mod linreg;

/// Nonlinear least-squares regression (gradient descent; no LAPACK).
pub mod nlinreg;

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
pub use basic::{
    cv,
    mad,
    mean,
    median,
    percentile,
    std_dev,
    std_dev_sample,
};
pub use cluster::{
    KMeansConfig,
    KMeansResult,
    cluster_sizes,
    compute_inertia,
    kmeans,
    kmeans_predict,
};
pub use correlation::{
    correlation_matrix,
    correlation_matrix_from_vec,
    pearson_correlation,
};
pub use distance::{
    chebyshev,
    cosine_distance,
    euclidean,
    manhattan,
};
pub use error_metrics::{
    RegressionReport,
    bias,
    mae,
    max_abs_error,
    mse,
    r2,
    regression_report,
    residuals,
    rmse,
};
pub use linreg::{
    LinearModel,
    elastic_net,
    l1,
    lasso,
    soft_threshold,
};
#[cfg(feature = "linalg")]
pub use linreg::{
    l2,
    ols,
    ridge,
};
pub use nlinreg::{
    NonlinearFitResult,
    nonlinear_least_squares,
    nonlinear_least_squares_design,
};
pub use variability::{
    mean_successive_differences,
    rmssd,
    sd_successive_differences,
    successive_differences,
};

// Version info
/// Current version of the `symworx-stats` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
