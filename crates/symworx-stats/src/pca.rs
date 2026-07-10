// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Principal Component Analysis (PCA)
//!
//! Uses SVD internally for maximum numerical stability.

use ndarray::{
    Array1,
    Array2,
    Axis,
    s,
};

use crate::svd::Svd;

/// Fitted Principal Component Analysis model.
#[derive(Debug, Clone)]
pub struct Pca {
    /// Principal components (right singular vectors)
    pub components: Array2<f64>,
    /// Explained variance for each component
    pub explained_variance: Array1<f64>,
    /// Mean of the training data (used for centering new data)
    pub mean: Array1<f64>,
    /// Number of components.
    pub n_components: usize,
}

impl Pca {
    /// Fit PCA model using SVD (numerically stable method).
    pub fn fit(data: &Array2<f64>, n_components: usize) -> Self {
        let _n_samples = data.nrows();

        // Center the data
        let mean = data.mean_axis(Axis(0)).expect("Data must not be empty");
        let centered = data - &mean;

        // Compute SVD
        let svd = Svd::compute(&centered);

        // Select top components
        let components = svd.vt.slice(s![0..n_components, ..]).t().to_owned();

        // Use population variance convention (s^2 / n) to match var_axis(0.0) in whitening tests
        let n_samples = centered.nrows() as f64;
        let explained_variance = if n_samples > 0.0 {
            svd.s
                .slice(s![0..n_components])
                .mapv(|s| (s * s) / n_samples)
        } else {
            Array1::zeros(n_components)
        };

        Self {
            components,
            explained_variance,
            mean,
            n_components,
        }
    }

    /// Transform new data into the principal component space.
    pub fn transform(&self, data: &Array2<f64>) -> Array2<f64> {
        let centered = data - &self.mean;
        centered.dot(&self.components)
    }

    /// Returns the explained variance ratio for each component (relative to total variance
    /// captured by the fitted components).
    pub fn explained_variance_ratio(&self) -> Array1<f64> {
        let total = self.explained_variance.sum();
        if total < 1e-12 {
            Array1::zeros(self.explained_variance.len())
        } else {
            &self.explained_variance / total
        }
    }

    /// Returns the cumulative explained variance ratio.
    pub fn cumulative_explained_variance_ratio(&self) -> Array1<f64> {
        let ratios = self.explained_variance_ratio();
        let mut cum = Array1::zeros(self.n_components());
        let mut sum = 0.0;
        for (i, &val) in ratios.iter().enumerate() {
            sum += val;
            cum[i] = sum;
        }
        cum
    }

    /// Number of components in this model.
    pub fn n_components(&self) -> usize {
        self.n_components
    }

    /// Transform data with whitening.
    pub fn transform_whitened(&self, data: &Array2<f64>) -> Array2<f64> {
        let transformed = self.transform(data);
        let std_dev = self.explained_variance.mapv(f64::sqrt);

        transformed / &std_dev.insert_axis(Axis(0))
    }
}

// TESTS
#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

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

    #[test]
    fn test_explained_variance_methods() {
        let data = array![[1.0, 2.0], [2.0, 4.0], [3.0, 6.0], [4.0, 8.0]];
        let pca = Pca::fit(&data, 2);

        let ratios = pca.explained_variance_ratio();
        let cum_ratios = pca.cumulative_explained_variance_ratio();

        assert_eq!(ratios.len(), 2);
        assert_eq!(cum_ratios.len(), 2);
        assert!((cum_ratios[1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_whitening() {
        let data = array![[1.0, 3.0], [2.0, 5.0], [3.0, 7.0]];
        let pca = Pca::fit(&data, 1);

        let whitened = pca.transform_whitened(&data);

        // After whitening, variance should be close to 1
        let var = whitened.var_axis(Axis(0), 0.0);
        assert!((var[0] - 1.0).abs() < 0.1);
    }
}
