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
//! Lasso / Elastic Net, logistic (binary + OVR), Gaussian NB, k-NN, rule lists,
//! classification metrics (incl. ROC/AUC), preprocessing, and k-means do
//! **not** require `linalg`. LDA fit and polynomial degree search require
//! `linalg`.

#![allow(unused_imports)]
#![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/symworx-stats")]

// Public API
/// Autocorrelation functions.
pub mod autocorrelation;

/// Basic statistics, including mean, median, mad.
pub mod basic;

/// Classification metrics (accuracy, confusion matrix, F1, …).
pub mod classification_metrics;

/// Clustering (k-means). Does not require the `linalg` feature.
pub mod cluster;

/// Correlation functions.
pub mod correlation;

/// Distance metrics (e.g., Euclidean).
pub mod distance;

/// Predicted-vs-expected error metrics (MAE, RMSE, R², residuals, regression report).
pub mod error_metrics;

/// Univariate histogram and Gaussian KDE (for any 1-D sample, incl. residuals).
pub mod density;

/// Model comparison / selection (AIC, BIC, nested χ² / F, adjusted R²).
pub mod model_select;

/// Feature preprocessing (standardize, min–max). Fit on train only.
pub mod preprocess;

/// Linear regression models (OLS, Ridge, Lasso, Elastic Net).
///
/// Closed-form `l2` / `ols` / `ridge` require the `linalg` feature.
/// Coordinate-descent `l1` / `lasso` / `elastic_net` do not.
pub mod linreg;

/// Binary logistic regression (gradient descent; no LAPACK).
pub mod logistic;

/// Linear Discriminant Analysis (fit needs `linalg`; predict is pure linear).
pub mod lda;

/// Gaussian Naive Bayes (pure Rust).
pub mod naive_bayes;

/// k-nearest neighbors multiclass classifier (stores training data).
pub mod knn;

/// Threshold rules and ordered rule-list classifiers (embed-friendly).
pub mod rules;

/// Nonlinear least-squares regression (gradient descent; no LAPACK).
pub mod nlinreg;

/// Univariate polynomial regression and degree search (`linalg` for fits).
pub mod polyreg;

#[cfg(feature = "linalg")]
/// Principal component analysis (requires `linalg` feature).
pub mod pca;

#[cfg(feature = "linalg")]
/// Singular value decomposition (requires `linalg` feature).
pub mod svd;

/// Index-based train/test splits and optional training folds.
pub mod split;

/// Teaching / demo synthetic tabular generators (StatsSym).
pub mod synthetic;

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
pub use classification_metrics::{
    ClassificationReport,
    RocCurve,
    accuracy,
    balanced_accuracy,
    binary_precision_recall_f1,
    classification_report,
    classification_report_binary_f64,
    confusion_matrix,
    f1_per_class,
    labels_from_binary_f64,
    macro_average,
    n_classes_from_labels,
    precision_per_class,
    recall_per_class,
    roc_auc,
    roc_auc_ovr,
    roc_curve,
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
pub use density::{
    HistBin,
    HistKde,
    Histogram,
    HistogramConfig,
    KdeConfig,
    KdeEstimate,
    hist_kde,
    hist_kde_with,
    histogram,
    histogram_default,
    kde_gaussian,
    kde_gaussian_default,
    silverman_bandwidth,
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
    residual_errors,
    residuals,
    rmse,
};
pub use knn::{
    KnnClassifier,
    KnnConfig,
    KnnMetric,
    knn_classify,
};
pub use lda::{
    LdaModel,
    lda,
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
pub use logistic::{
    LogisticConfig,
    LogisticModel,
    MulticlassLogisticModel,
    logistic,
    logistic_ovr,
    logistic_regression,
    logistic_regression_ovr,
    sigmoid,
};
pub use model_select::{
    ModelFitScores,
    NestedModelTest,
    adjusted_r2,
    aic_gaussian,
    bic_gaussian,
    chi2_sf,
    nested_f_stat,
    nested_lr_chi2,
    rss,
};
pub use naive_bayes::{
    GaussianNb,
    GaussianNbConfig,
    gaussian_nb,
    gaussian_nb_default,
};
pub use nlinreg::{
    NonlinearFitResult,
    nonlinear_least_squares,
    nonlinear_least_squares_design,
};
pub use polyreg::{
    PolyRegError,
    PolynomialDegreeFit,
    PolynomialDegreeSearch,
    PolynomialSearchConfig,
    fit_polynomial_degrees,
    fit_polynomial_degrees_with,
    max_feasible_degree,
    polynomial_design,
    soft_min_samples_for_degree,
};
pub use preprocess::{
    MinMaxScaler,
    StandardScaler,
};
pub use rules::{
    ClassificationRule,
    Comparison,
    DecisionStump,
    RuleHit,
    RuleListClassifier,
    ThresholdCondition,
    fit_decision_stump,
};
pub use split::{
    MIN_SPLIT_FRACTION,
    MIN_SPLIT_SAMPLES,
    SplitConfig,
    SplitError,
    SplitPart,
    TrainTestSplit,
    max_train_folds,
    min_split_size,
    repeated_train_test_split,
    take_indices,
    take_indices_cloned,
    train_test_split,
};
pub use synthetic::{
    SyntheticError,
    SyntheticPreset,
    SyntheticSpec,
    SyntheticTable,
    generate as generate_synthetic,
    generate_default as generate_synthetic_default,
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
