// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Principal Component Analysis (PCA)
//!
//! Dimensionality reduction technique that transforms data into a new
//! coordinate system where the greatest variance lies on the first components.

use ndarray::{Array1, Array2, Axis, s};
use ndarray_linalg::{Eigh, UPLO};

/// Fitted Principal Component Analysis model.
#[derive(Debug, Clone)]
pub struct Pca {
    /// Principal components (eigenvectors), shape `(n_features, n_components)`
    pub components: Array2<f64>,
    /// Explained variance for each component
    pub explained_variance: Array1<f64>,
    /// Mean of the training data (used for centering)
    pub mean: Array1<f64>,
}

impl Pca {
    /// Fit a PCA model to the data.
    ///
    /// # Arguments
    /// * `data` - 2D array of shape `(n_samples, n_features)`
    /// * `n_components` - Number of principal components to keep
    ///
    /// # Panics
    /// Panics if eigen-decomposition fails.
    pub fn fit(data: &Array2<f64>, n_components: usize) -> Self {
        let n_samples = data.nrows();

        // Compute mean per feature and center the data
        let mean = data.mean_axis(Axis(0)).expect("Data must not be empty");
        let centered = data - &mean;

        // Compute covariance matrix
        let cov = centered.t().dot(&centered) / ((n_samples - 1) as f64);

        // Eigen decomposition
        let (eigenvalues, eigenvectors) = cov
            .eigh(UPLO::Upper)
            .expect("Eigen decomposition failed");

        // Sort by descending eigenvalues
        let mut idx: Vec<usize> = (0..eigenvalues.len()).collect();
        idx.sort_by(|&a, &b| eigenvalues[b].partial_cmp(&eigenvalues[a]).unwrap());

        // Select top n_components
        let components = eigenvectors.select(Axis(1), &idx[..n_components]);
        let explained_variance = eigenvalues.select(Axis(0), &idx[..n_components]);

        Self {
            components,
            explained_variance,
            mean,
        }
    }

    /// Transform data using the fitted PCA model.
    ///
    /// # Arguments
    /// * `data` - 2D array of shape `(n_samples, n_features)`
    ///
    /// # Returns
    /// Transformed data of shape `(n_samples, n_components)`
    pub fn transform(&self, data: &Array2<f64>) -> Array2<f64> {
        let centered = data - &self.mean;
        centered.dot(&self.components)
    }
}


// TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    #[test]
    fn test_pca_basic() {
        let data = array![
            [2.5, 2.4],
            [0.5, 0.7],
            [2.2, 2.9],
            [1.9, 2.2],
            [3.1, 3.0],
            [2.3, 2.7],
            [2.0, 1.6],
            [1.0, 1.1],
            [1.5, 1.6],
            [1.1, 0.9],
        ];

        let pca = Pca::fit(&data, 1);
        let transformed = pca.transform(&data);

        assert_eq!(transformed.shape(), &[10, 1]);
        assert!(pca.explained_variance[0] > 0.0);
    }
}
