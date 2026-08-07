// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Feature preprocessing for predictive models.
//!
//! Fit transforms **only on training rows**, then apply to validation / test
//! (avoids leakage in CV and hold-out pipelines). Pure Rust — no `linalg`.
//!
//! Embed note: after `fit`, ship `mean` and `scale` with the model coefficients
//! so device-side inference can standardize with a multiply-add only.
//! Full recipes (C, Swift, Kotlin, TS): `docs/model_export.md`.

use ndarray::{
    Array1,
    Array2,
    Axis,
};

/// Column-wise zero-mean, unit-variance scaler (sklearn-style `StandardScaler`
/// with population std, `ddof = 0`).
///
/// Columns with zero (or tiny) variance get `scale = 1.0` so transform is a
/// no-op divide for that feature.
#[derive(Debug, Clone, PartialEq)]
pub struct StandardScaler {
    /// Per-feature mean (length = n_features).
    pub mean: Array1<f64>,
    /// Per-feature scale (std); never zero.
    pub scale: Array1<f64>,
}

impl StandardScaler {
    /// Number of features.
    pub fn n_features(&self) -> usize {
        self.mean.len()
    }

    /// Fit means and scales on `x` (n_samples × n_features).
    ///
    /// # Panics
    /// Panics if `x` has zero rows or zero columns.
    pub fn fit(x: &Array2<f64>) -> Self {
        assert!(x.nrows() > 0, "StandardScaler::fit needs at least one row");
        assert!(x.ncols() > 0, "StandardScaler::fit needs at least one column");

        let mean = x.mean_axis(Axis(0)).expect("mean_axis on non-empty array");
        let n = x.nrows() as f64;
        let mut scale = Array1::<f64>::zeros(x.ncols());
        for j in 0..x.ncols() {
            let mut ss = 0.0;
            let m = mean[j];
            for i in 0..x.nrows() {
                let d = x[[i, j]] - m;
                ss += d * d;
            }
            let std = (ss / n).sqrt();
            // Floor: constant columns → identity scale
            scale[j] = if std < 1e-12 { 1.0 } else { std };
        }
        Self { mean, scale }
    }

    /// Apply `(x − mean) / scale` with the fitted parameters.
    ///
    /// # Panics
    /// Panics if `x.ncols()` does not match the fitted feature count.
    pub fn transform(&self, x: &Array2<f64>) -> Array2<f64> {
        assert_eq!(
            x.ncols(),
            self.n_features(),
            "feature dimension mismatch: X has {} cols, scaler has {}",
            x.ncols(),
            self.n_features()
        );
        let mut out = x.clone();
        for j in 0..x.ncols() {
            let m = self.mean[j];
            let s = self.scale[j];
            for i in 0..x.nrows() {
                out[[i, j]] = (x[[i, j]] - m) / s;
            }
        }
        out
    }

    /// Fit on `x` and return `(scaler, transformed_x)`.
    pub fn fit_transform(x: &Array2<f64>) -> (Self, Array2<f64>) {
        let scaler = Self::fit(x);
        let xt = scaler.transform(x);
        (scaler, xt)
    }

    /// Inverse of [`transform`]: `x * scale + mean`.
    pub fn inverse_transform(&self, x: &Array2<f64>) -> Array2<f64> {
        assert_eq!(
            x.ncols(),
            self.n_features(),
            "feature dimension mismatch in inverse_transform"
        );
        let mut out = x.clone();
        for j in 0..x.ncols() {
            let m = self.mean[j];
            let s = self.scale[j];
            for i in 0..x.nrows() {
                out[[i, j]] = x[[i, j]] * s + m;
            }
        }
        out
    }

    /// Standardize a single feature row (for embedded / streaming inference).
    pub fn transform_row(&self, row: &[f64]) -> Vec<f64> {
        assert_eq!(row.len(), self.n_features());
        row.iter()
            .enumerate()
            .map(|(j, &v)| (v - self.mean[j]) / self.scale[j])
            .collect()
    }
}

/// Min–max scale columns to `[0, 1]` (or a custom range).
///
/// Constant columns map to the midpoint of the output range.
#[derive(Debug, Clone, PartialEq)]
pub struct MinMaxScaler {
    /// Per-feature minimum on the training data.
    pub data_min: Array1<f64>,
    /// Per-feature maximum on the training data.
    pub data_max: Array1<f64>,
    /// Output range low (default 0).
    pub feature_range: (f64, f64),
}

impl MinMaxScaler {
    /// Fit column min/max on `x`.
    pub fn fit(x: &Array2<f64>, feature_range: (f64, f64)) -> Self {
        assert!(x.nrows() > 0 && x.ncols() > 0);
        assert!(feature_range.1 > feature_range.0, "feature_range high must be > low");
        let mut data_min = Array1::zeros(x.ncols());
        let mut data_max = Array1::zeros(x.ncols());
        for j in 0..x.ncols() {
            let col = x.column(j);
            let mut mn = f64::INFINITY;
            let mut mx = f64::NEG_INFINITY;
            for &v in col.iter() {
                mn = mn.min(v);
                mx = mx.max(v);
            }
            data_min[j] = mn;
            data_max[j] = mx;
        }
        Self {
            data_min,
            data_max,
            feature_range,
        }
    }

    /// Fit with default output range `[0, 1]`.
    pub fn fit_01(x: &Array2<f64>) -> Self {
        Self::fit(x, (0.0, 1.0))
    }

    /// Apply min–max scaling.
    pub fn transform(&self, x: &Array2<f64>) -> Array2<f64> {
        assert_eq!(x.ncols(), self.data_min.len());
        let (lo, hi) = self.feature_range;
        let mut out = Array2::<f64>::zeros(x.raw_dim());
        for j in 0..x.ncols() {
            let mn = self.data_min[j];
            let mx = self.data_max[j];
            let span = mx - mn;
            for i in 0..x.nrows() {
                if span < 1e-12 {
                    out[[i, j]] = 0.5 * (lo + hi);
                } else {
                    out[[i, j]] = (x[[i, j]] - mn) / span * (hi - lo) + lo;
                }
            }
        }
        out
    }

    /// Fit and transform in one call.
    pub fn fit_transform(x: &Array2<f64>, feature_range: (f64, f64)) -> (Self, Array2<f64>) {
        let s = Self::fit(x, feature_range);
        let xt = s.transform(x);
        (s, xt)
    }
}

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    #[test]
    fn standard_scaler_zero_mean_unit_var() {
        let x = array![[1.0, 100.0], [2.0, 200.0], [3.0, 300.0], [4.0, 400.0],];
        let (sc, z) = StandardScaler::fit_transform(&x);
        for j in 0..2 {
            let mean_j: f64 = z.column(j).mean().unwrap();
            assert!(mean_j.abs() < 1e-12, "mean col {j} = {mean_j}");
            let mut ss = 0.0;
            for i in 0..z.nrows() {
                ss += z[[i, j]] * z[[i, j]];
            }
            let var = ss / z.nrows() as f64;
            assert!((var - 1.0).abs() < 1e-12, "var col {j} = {var}");
        }
        // inverse recovers original
        let back = sc.inverse_transform(&z);
        for i in 0..x.nrows() {
            for j in 0..x.ncols() {
                assert!((back[[i, j]] - x[[i, j]]).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn constant_column_scale_one() {
        let x = array![[5.0, 1.0], [5.0, 2.0], [5.0, 3.0]];
        let sc = StandardScaler::fit(&x);
        assert!((sc.scale[0] - 1.0).abs() < 1e-15);
        let z = sc.transform(&x);
        // constant col → all zeros after centering
        assert!(z.column(0).iter().all(|&v| v.abs() < 1e-15));
    }

    #[test]
    fn transform_row_matches_matrix() {
        let x = array![[0.0, 10.0], [2.0, 20.0], [4.0, 30.0]];
        let sc = StandardScaler::fit(&x);
        let z = sc.transform(&x);
        let row = sc.transform_row(&[2.0, 20.0]);
        assert!((row[0] - z[[1, 0]]).abs() < 1e-12);
        assert!((row[1] - z[[1, 1]]).abs() < 1e-12);
    }

    #[test]
    fn minmax_01() {
        let x = array![[0.0, 5.0], [10.0, 15.0]];
        let sc = MinMaxScaler::fit_01(&x);
        let z = sc.transform(&x);
        assert!((z[[0, 0]] - 0.0).abs() < 1e-12);
        assert!((z[[1, 0]] - 1.0).abs() < 1e-12);
        assert!((z[[0, 1]] - 0.0).abs() < 1e-12);
        assert!((z[[1, 1]] - 1.0).abs() < 1e-12);
    }
}
