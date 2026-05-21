// symworx/crates/symworx-stats/src/pca.rs
// Copyright (C) 2026 cSYMd, All rights reserved.
// 

use ndarray::{Array1, Array2, Axis};
use ndarray_linalg::{Eigh, UPLO};

// ==========================================================
// Principal Component Analysis 
// ==========================================================
pub struct Pca {
    pub components: Array2<f64>,
    pub explained_variance: Array1<f64>,
    pub mean: Array1<f64>,
}

/// Principal Component Analysis (PCA) implementation
/// PCA is a dimensionality reduction technique that transforms data to a new coordinate system
/// such that the greatest variance by any projection of the data comes to lie on the first coordinate
///
/// # Aguments
/// * `data` - A 2D array where rows are samples and columns are features
/// * `n_components` - The number of principal components to compute
///
/// # Returns
/// A `Pca` struct containing the principal components, explained variance, and mean of the
/// original data.
impl Pca {
    pub fn fit(data: &Array2<f64>, n_components: usize) -> Self {
        // Compute column means
        let mean = data.mean_axis(Axis(0)).unwrap();

        // Center data
        let centered = data - &mean;

        // Covariance matrix
        let cov = centered.t().dot(&centered) / ((data.nrows() - 1) as f64);

        // Eigen decomposition
        let (eigenvalues, eigenvectors) = cov.eigh(UPLO::Upper).unwrap();

        // Sort eigenvalues descending
        let mut idx: Vec<usize> = (0..eigenvalues.len()).collect();
        idx.sort_by(|&i, &j| eigenvalues[j].partial_cmp(&eigenvalues[i]).unwrap());

        // Select top components
        let components = eigenvectors.select(Axis(1), &idx[..n_components]);
        let explained_variance = eigenvalues.select(Axis(0), &idx[..n_components]);

        Self {
            components,
            explained_variance,
            mean,
        }
    }

    pub fn transform(&self, data: &Array2<f64>) -> Array2<f64> {
        let centered = data - &self.mean;
        centered.dot(&self.components)
    }
}


// ==========================================================
// TESTS
// ==========================================================
#[cfg(test)]
mod test_pca {
    use super::*;
    use ndarray::array;

    #[test]
    fn test_pca() {
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
        let _transformed = pca.transform(&data);
    }
}
