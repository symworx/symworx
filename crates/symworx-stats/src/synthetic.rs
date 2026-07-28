// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Teaching / demo synthetic tabular data for StatsSym and examples.
//!
//! Presets produce named columns suitable for describe, correlate, regress,
//! classify, and cluster lab tasks. All generators are pure Rust (no LAPACK).

use rand::{
    Rng,
    SeedableRng,
    rngs::StdRng,
};
use symworx_math::random::sample as dist;

/// Named teaching presets (univariate and multivariate).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntheticPreset {
    /// Single normal column `x`.
    Normal1D,
    /// Two correlated normals `x`, `y` (Pearson ≈ ρ).
    BivariateCorrelated,
    /// Linear model `y = β0 + β1 x1 + … + noise`; columns `x1..xp`, `y`.
    LinearRegression,
    /// Two Gaussian blobs in 2D + `label` ∈ {0,1}.
    TwoClassBlobs,
    /// Three Gaussian blobs in 2D + `label` ∈ {0,1,2}.
    ThreeClassBlobs,
    /// Three clusters in 2D (no label column; for k-means demos).
    Cluster3,
}

impl SyntheticPreset {
    /// Stable list for menus (order shown in TUI).
    pub const ALL: [SyntheticPreset; 6] = [
        SyntheticPreset::Normal1D,
        SyntheticPreset::BivariateCorrelated,
        SyntheticPreset::LinearRegression,
        SyntheticPreset::TwoClassBlobs,
        SyntheticPreset::ThreeClassBlobs,
        SyntheticPreset::Cluster3,
    ];

    /// Short menu label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal1D => "Normal 1D",
            Self::BivariateCorrelated => "Bivariate correlated",
            Self::LinearRegression => "Linear regression",
            Self::TwoClassBlobs => "Two-class blobs",
            Self::ThreeClassBlobs => "Three-class blobs",
            Self::Cluster3 => "3 clusters (k-means)",
        }
    }

    /// One-line teaching note.
    pub fn description(self) -> &'static str {
        match self {
            Self::Normal1D => "Univariate Normal(μ,σ) — describe, histogram",
            Self::BivariateCorrelated => "Two normals with correlation ρ — scatter, Pearson",
            Self::LinearRegression => "y = β·x + noise — OLS, residuals, R²",
            Self::TwoClassBlobs => "Two 2D blobs + labels — logistic, k-NN, ROC",
            Self::ThreeClassBlobs => "Three 2D blobs + labels — multiclass classifiers",
            Self::Cluster3 => "Three unlabeled 2D blobs — k-means",
        }
    }
}

/// Tunable knobs shared across presets (unused fields are ignored per preset).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyntheticSpec {
    /// Number of rows (samples). Default 200.
    pub n: usize,
    /// RNG seed for reproducibility. Default 42.
    pub seed: u64,
    /// Additive noise scale (regression / separation softness). Default 0.5.
    pub noise: f64,
    /// Mean for Normal1D. Default 0.
    pub mean: f64,
    /// Std-dev for Normal1D / bivariate margins. Default 1.
    pub std_dev: f64,
    /// Target correlation for BivariateCorrelated in (-1, 1). Default 0.7.
    pub rho: f64,
    /// Number of predictors for LinearRegression (excluding intercept). Default 2.
    pub n_features: usize,
    /// Class / cluster center separation. Default 2.5.
    pub separation: f64,
}

impl Default for SyntheticSpec {
    fn default() -> Self {
        Self {
            n: 200,
            seed: 42,
            noise: 0.5,
            mean: 0.0,
            std_dev: 1.0,
            rho: 0.7,
            n_features: 2,
            separation: 2.5,
        }
    }
}

/// Generated table (column-major) ready for CSV export or StatsSym session.
#[derive(Debug, Clone, PartialEq)]
pub struct SyntheticTable {
    /// Column names (aligned with [`Self::columns`]).
    pub headers: Vec<String>,
    /// One vector per column; each length `n`.
    pub columns: Vec<Vec<f64>>,
    /// Index of target / label column when supervised, else `None`.
    pub target_col: Option<usize>,
    /// Ground-truth description for teaching UIs.
    pub notes: String,
    /// Which preset produced this table.
    pub preset: SyntheticPreset,
    /// Spec used at generation time.
    pub spec: SyntheticSpec,
}

impl SyntheticTable {
    /// Number of samples (rows).
    pub fn n_rows(&self) -> usize {
        self.columns.first().map(|c| c.len()).unwrap_or(0)
    }

    /// Number of columns.
    pub fn n_cols(&self) -> usize {
        self.columns.len()
    }
}

/// Errors from synthetic generation (invalid parameters).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticError(pub String);

impl std::fmt::Display for SyntheticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for SyntheticError {}

/// Generate a synthetic teaching table for `preset` with `spec`.
pub fn generate(preset: SyntheticPreset, spec: &SyntheticSpec) -> Result<SyntheticTable, SyntheticError> {
    if spec.n == 0 {
        return Err(SyntheticError("n must be > 0".into()));
    }
    if !(spec.std_dev.is_finite() && spec.std_dev > 0.0) {
        return Err(SyntheticError("std_dev must be finite and > 0".into()));
    }
    if !(spec.noise.is_finite() && spec.noise >= 0.0) {
        return Err(SyntheticError("noise must be finite and >= 0".into()));
    }
    let mut rng = StdRng::seed_from_u64(spec.seed);
    match preset {
        SyntheticPreset::Normal1D => gen_normal_1d(&mut rng, spec),
        SyntheticPreset::BivariateCorrelated => gen_bivariate(&mut rng, spec),
        SyntheticPreset::LinearRegression => gen_linear_regression(&mut rng, spec),
        SyntheticPreset::TwoClassBlobs => gen_class_blobs(&mut rng, spec, 2),
        SyntheticPreset::ThreeClassBlobs => gen_class_blobs(&mut rng, spec, 3),
        SyntheticPreset::Cluster3 => gen_cluster3(&mut rng, spec),
    }
}

fn gen_normal_1d(rng: &mut StdRng, spec: &SyntheticSpec) -> Result<SyntheticTable, SyntheticError> {
    let mut x = Vec::with_capacity(spec.n);
    for _ in 0..spec.n {
        x.push(dist::normal(rng, spec.mean, spec.std_dev));
    }
    Ok(SyntheticTable {
        headers: vec!["x".into()],
        columns: vec![x],
        target_col: None,
        notes: format!(
            "Normal1D: x ~ N(μ={:.3}, σ={:.3}), n={}, seed={}",
            spec.mean, spec.std_dev, spec.n, spec.seed
        ),
        preset: SyntheticPreset::Normal1D,
        spec: *spec,
    })
}

fn gen_bivariate(rng: &mut StdRng, spec: &SyntheticSpec) -> Result<SyntheticTable, SyntheticError> {
    let rho = spec.rho.clamp(-0.999, 0.999);
    let mut x = Vec::with_capacity(spec.n);
    let mut y = Vec::with_capacity(spec.n);
    let s = spec.std_dev;
    for _ in 0..spec.n {
        let z1 = dist::normal(rng, 0.0, 1.0);
        let z2 = dist::normal(rng, 0.0, 1.0);
        let xi = spec.mean + s * z1;
        let yi = spec.mean + s * (rho * z1 + (1.0 - rho * rho).sqrt() * z2);
        x.push(xi);
        y.push(yi);
    }
    Ok(SyntheticTable {
        headers: vec!["x".into(), "y".into()],
        columns: vec![x, y],
        target_col: None,
        notes: format!(
            "Bivariate: x,y ~ correlated normals ρ≈{:.2}, σ={:.3}, n={}, seed={}",
            rho, s, spec.n, spec.seed
        ),
        preset: SyntheticPreset::BivariateCorrelated,
        spec: *spec,
    })
}

fn gen_linear_regression(rng: &mut StdRng, spec: &SyntheticSpec) -> Result<SyntheticTable, SyntheticError> {
    let p = spec.n_features.clamp(1, 8);
    // Fixed teaching coefficients: β0=1, βj = j
    let mut headers = Vec::with_capacity(p + 1);
    let mut columns: Vec<Vec<f64>> = Vec::with_capacity(p + 1);
    for j in 1..=p {
        headers.push(format!("x{j}"));
        columns.push(Vec::with_capacity(spec.n));
    }
    headers.push("y".into());
    columns.push(Vec::with_capacity(spec.n));

    for _ in 0..spec.n {
        let mut y = 1.0; // intercept
        for j in 0..p {
            let xj = dist::normal(rng, 0.0, 1.0);
            columns[j].push(xj);
            y += (j + 1) as f64 * xj;
        }
        y += dist::normal(rng, 0.0, spec.noise.max(1e-9));
        columns[p].push(y);
    }

    Ok(SyntheticTable {
        headers,
        columns,
        target_col: Some(p),
        notes: format!(
            "LinearRegression: y=1+Σ j·x_j + N(0,{:.3}), p={}, n={}, seed={}",
            spec.noise, p, spec.n, spec.seed
        ),
        preset: SyntheticPreset::LinearRegression,
        spec: *spec,
    })
}

fn gen_class_blobs(rng: &mut StdRng, spec: &SyntheticSpec, n_classes: usize) -> Result<SyntheticTable, SyntheticError> {
    let sep = spec.separation.max(0.1);
    // Place centers on a circle
    let mut centers = Vec::with_capacity(n_classes);
    for k in 0..n_classes {
        let ang = 2.0 * std::f64::consts::PI * k as f64 / n_classes as f64;
        centers.push((sep * ang.cos(), sep * ang.sin()));
    }
    let sigma = (spec.noise.max(0.1)).max(0.15);

    let mut x1 = Vec::with_capacity(spec.n);
    let mut x2 = Vec::with_capacity(spec.n);
    let mut label = Vec::with_capacity(spec.n);
    for i in 0..spec.n {
        let k = i % n_classes;
        let (cx, cy) = centers[k];
        x1.push(dist::normal(rng, cx, sigma));
        x2.push(dist::normal(rng, cy, sigma));
        label.push(k as f64);
    }

    Ok(SyntheticTable {
        headers: vec!["x1".into(), "x2".into(), "label".into()],
        columns: vec![x1, x2, label],
        target_col: Some(2),
        notes: format!(
            "{}: {} blobs, separation={:.2}, noise σ={:.2}, n={}, seed={}",
            if n_classes == 2 {
                "TwoClassBlobs"
            } else {
                "ThreeClassBlobs"
            },
            n_classes,
            sep,
            sigma,
            spec.n,
            spec.seed
        ),
        preset: if n_classes == 2 {
            SyntheticPreset::TwoClassBlobs
        } else {
            SyntheticPreset::ThreeClassBlobs
        },
        spec: *spec,
    })
}

fn gen_cluster3(rng: &mut StdRng, spec: &SyntheticSpec) -> Result<SyntheticTable, SyntheticError> {
    // Same geometry as three-class but without label column
    let mut t = gen_class_blobs(rng, spec, 3)?;
    t.headers = vec!["x1".into(), "x2".into()];
    t.columns.truncate(2);
    t.target_col = None;
    t.preset = SyntheticPreset::Cluster3;
    t.notes = format!(
        "Cluster3: three unlabeled blobs, separation={:.2}, n={}, seed={}",
        spec.separation, spec.n, spec.seed
    );
    Ok(t)
}

/// Convenience: default-spec generate (seed 42, n=200).
pub fn generate_default(preset: SyntheticPreset) -> Result<SyntheticTable, SyntheticError> {
    generate(preset, &SyntheticSpec::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_1d_shape() {
        let t = generate_default(SyntheticPreset::Normal1D).unwrap();
        assert_eq!(t.n_cols(), 1);
        assert_eq!(t.n_rows(), 200);
        assert!(t.target_col.is_none());
    }

    #[test]
    fn bivariate_positive_correlation() {
        let spec = SyntheticSpec {
            n: 500,
            rho: 0.8,
            ..Default::default()
        };
        let t = generate(SyntheticPreset::BivariateCorrelated, &spec).unwrap();
        let x = &t.columns[0];
        let y = &t.columns[1];
        let n = x.len() as f64;
        let mx = x.iter().sum::<f64>() / n;
        let my = y.iter().sum::<f64>() / n;
        let mut num = 0.0;
        let mut dx = 0.0;
        let mut dy = 0.0;
        for i in 0..x.len() {
            let a = x[i] - mx;
            let b = y[i] - my;
            num += a * b;
            dx += a * a;
            dy += b * b;
        }
        let r = num / (dx.sqrt() * dy.sqrt());
        assert!(r > 0.6, "expected strong positive corr, got {r}");
    }

    #[test]
    fn linear_regression_has_target() {
        let t = generate_default(SyntheticPreset::LinearRegression).unwrap();
        assert_eq!(t.target_col, Some(t.n_cols() - 1));
        assert!(t.n_cols() >= 3);
    }

    #[test]
    fn two_class_labels_binary() {
        let t = generate_default(SyntheticPreset::TwoClassBlobs).unwrap();
        let labels = &t.columns[2];
        assert!(labels.iter().all(|&v| v == 0.0 || v == 1.0));
    }

    #[test]
    fn seed_reproducible() {
        let a = generate_default(SyntheticPreset::Normal1D).unwrap();
        let b = generate_default(SyntheticPreset::Normal1D).unwrap();
        assert_eq!(a.columns, b.columns);
    }

    #[test]
    fn invalid_n_errors() {
        let spec = SyntheticSpec {
            n: 0,
            ..Default::default()
        };
        assert!(generate(SyntheticPreset::Normal1D, &spec).is_err());
    }
}
