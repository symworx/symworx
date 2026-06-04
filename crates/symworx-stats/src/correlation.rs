// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

use ndarray::{
    Array2,
    ArrayView1,
    Axis,
};

/// Computes Pearson correlation coefficient between two columns.
/// # Arguments
/// * `data` - 2D array with **rows = observations**, **columns =
///   variables**
/// * `col1` - index of first variable (column)
/// * `col2` - index of second variable (column)
///
/// # Returns
/// Pearson correlation coefficient (r) between -1.0 and 1.0
pub fn pearson_correlation(data: &Array2<f64>, col1: usize, col2: usize) -> f64 {
    let n = data.nrows();
    if n < 2 || col1 >= data.ncols() || col2 >= data.ncols() {
        return 0.0;
    }

    let x = data.column(col1);
    let y = data.column(col2);

    let mean_x = x.mean().unwrap_or(0.0);
    let mean_y = y.mean().unwrap_or(0.0);

    let mut numerator = 0.0;
    let mut sum_sq_x = 0.0;
    let mut sum_sq_y = 0.0;

    for i in 0..n {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        numerator += dx * dy;
        sum_sq_x += dx * dx;
        sum_sq_y += dy * dy;
    }

    let denominator = (sum_sq_x * sum_sq_y).sqrt();
    if denominator < 1e-12 {
        return 0.0; // constant or near-constant
    }

    numerator / denominator
}

/// Generates a full Pearson correlation matrix for any number of variables (columns).
///
/// # Arguments
/// * `data` - 2D array with **rows = observations**, **columns = variables**
///
/// # Returns
/// Square correlation matrix (symmetric, diagonal = 1.0)
pub fn correlation_matrix(data: &Array2<f64>) -> Array2<f64> {
    let ncols = data.ncols();
    if ncols == 0 {
        return Array2::zeros((0, 0));
    }

    let mut corr = Array2::<f64>::zeros((ncols, ncols));

    for i in 0..ncols {
        corr[[i, i]] = 1.0;
        for j in (i + 1)..ncols {
            let r = pearson_correlation(data, i, j);
            corr[[i, j]] = r;
            corr[[j, i]] = r;
        }
    }
    corr
}

/// Convenience function: Create correlation matrix from a Vec of Vecs
pub fn correlation_matrix_from_vec(data: &[Vec<f64>]) -> Array2<f64> {
    if data.is_empty() || data[0].is_empty() {
        return Array2::zeros((0, 0));
    }

    let nrows = data.len();
    let ncols = data[0].len();

    // Check all rows have same length
    if !data.iter().all(|row| row.len() == ncols) {
        panic!("All rows must have the same number of columns");
    }

    let array = Array2::from_shape_vec((nrows, ncols), data.iter().flatten().cloned().collect())
        .expect("Failed to create Array2");

    correlation_matrix(&array)
}

// TESTS
#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    #[test]
    fn test_pearson_two_variables() {
        let data = array![[1.0, 2.0], [2.0, 4.0], [3.0, 6.0], [4.0, 8.0]];

        let r = pearson_correlation(&data, 0, 1);
        assert!((r - 1.0).abs() < 1e-8);
    }

    #[test]
    fn test_correlation_matrix_3_variables() {
        let data = array![
            [1.0, 2.0, 3.0],
            [2.0, 4.0, 6.0],
            [3.0, 6.0, 9.0],
            [4.0, 8.0, 12.0]
        ];

        let matrix = correlation_matrix(&data);
        println!("Correlation Matrix (3 variables):\n{}", matrix);

        assert!((matrix[[0, 1]] - 1.0).abs() < 1e-8);
        assert!((matrix[[0, 2]] - 1.0).abs() < 1e-8);
        assert!((matrix[[1, 2]] - 1.0).abs() < 1e-8);
    }

    #[test]
    fn test_from_vec_4_variables() {
        let data: Vec<Vec<f64>> = vec![
            vec![1.0, 3.0, 2.0, 4.0],
            vec![2.0, 5.0, 3.0, 8.0],
            vec![3.0, 7.0, 5.0, 11.0],
            vec![4.0, 9.0, 6.0, 15.0],
        ];

        let matrix = correlation_matrix_from_vec(&data);
        println!("Correlation Matrix from Vec (4 vars):\n{}", matrix);

        assert_eq!(matrix.nrows(), 4);
        assert_eq!(matrix.ncols(), 4);
    }
}
