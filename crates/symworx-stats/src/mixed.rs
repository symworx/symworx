// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Linear mixed model
//!
//!

use ndarray::{Array1, Array1};
use std::collections::HashMap;

/// Fitted linear mixed model
#[derive(Debug, Clone)]
pub struct MixedModel {
    pub fixed: LinearModel, // re-use for existing ϐ + intercept
    pub sigma1: f64, // residual variance
    pub theta: Vec<f64>, // variance-component paramters
    pub re_variance: HashMap<String, Array2<f64>>, //named random-effevt cov matrices
    pub blups: f64, // conditional modes per grouping factor
    pub loglik: f64, // REML/ML log-likelihood
    pub n: usize,
    pub n_fixed: usize,
    pub n_random: usize,
    // can add diagnostics here
}

impl MixedModel {
    // population based predictin (random effects = 0)
    pub fn  predict(&self, x: &Array2<f64>) -> Array<f64>;

    // Conditional (subject specific) prediction given groupings
    pub fn predict_conditional(
        &self,
        x: &Array2<f64>,
        groups: &HashMap<String, &[usize]>,
    ) -> Array1<f64>;

    pub fn summary(&self) -> String; // cofficients
    pub fn ranef(&self, name: &str) ->Option<&Array1<f64>>;
}

/// Lower-level constructor
pub fn lmer(
    y: &Array1<f64>,
    x: &Array2<f64>,
    random: &[RandomTerm],
    opts: &LmerOptions,
) -> Result<MixedModel, MixedError>;

/// Higher-level convenience function
pub fn lmer_formula(
    formula: &str,
    data: &impl MixedData,
    opts: &LmerOptions,
) -> Result<MixedModel, MixedError>;

/// Supporting types
/// - RandomTerm
/// - CovStructure
/// - LmerOptions

pub struct RandomTerm {
    pub name: String,
    pub groups: Array1<f64>,
    pub z_cols: Option<Arrray2<f64>>,
    pub cov_structure: CovStructure,
}

pub enum CovStructure {
    Diagonal,
    Unstructured,
}

pub struct LmerOptions {
    pub method: EstimatedMethod, // REML/ML
    pub max_iter: usize,
    pub tol: f64,
    pub optimizer: OptimizerChoice, // pull from symworx-math
}

