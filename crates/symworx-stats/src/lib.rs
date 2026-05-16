// core/statistics/mod.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

pub mod autocorrelation;
pub mod basic;
pub mod correlation;
pub mod distance;
pub mod errors;
pub mod linreg;
pub mod pca;
pub mod variability;

pub use autocorrelation::acf;
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
pub use distance::euclidean;
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
