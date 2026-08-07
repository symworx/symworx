// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Linear mixed models (single grouping factor).
//!
//! Fits Gaussian LMMs of the form
//!
//! ```text
//! y = Xβ + Z u + ε,
//! u_g ~ N(0, G),  ε ~ N(0, σ² I)
//! ```
//!
//! with **one** grouping factor. Supported random structures:
//!
//! * **Random intercept** — [`RandomTerm::random_intercept`] (`q = 1`)
//! * **Linear growth** — [`RandomTerm::linear_growth`] with `Z = [1, t]` and
//!   unstructured `2 × 2` `G` (intercept–slope covariance)
//! * **Diagonal growth** — [`RandomTerm::linear_growth_diagonal`]
//!
//! Variance components use relative covariance `Γ = G / σ²`, profiled
//! REML (default) or ML. The dense `n × n` `V` is never formed (Woodbury /
//! block form). Multi-parameter variance search uses multi-start gradient
//! descent ([`LmerConfig::n_restarts`]).
//!
//! Design helpers for time bases live in [`design`].
//!
//! Requires the **`linalg`** feature.
//!
//! # Design conventions
//!
//! - `x` is `n × p` **without** an intercept column when
//!   [`LmerConfig::fit_intercept`] is `true` (same as [`crate::ols`]).
//! - Group labels may be arbitrary `usize` values; they are remapped internally.
//! - `z_cols` is `n × q` (or `None` for a random intercept).
//!
//! # Example
//!
//! ```ignore
//! use symworx_stats::{lmer, LmerConfig, RandomTerm};
//! let term = RandomTerm::linear_growth("id", groups, &time)?;
//! let fit = lmer(&y, &x, &[term], &LmerConfig::default())?;
//! ```

mod design;
mod fit;
mod reml;
mod simulate;
mod types;

pub use design::{
    center_time,
    center_time_at,
    fixed_piecewise_linear,
    hstack,
    mean_time,
    piecewise_hinges,
    time_powers,
    try_hstack,
    z_intercept,
    z_intercept_slope,
    z_intercept_slope_centered,
};
pub use fit::{
    lmer,
    lmer_default,
};
pub use simulate::{
    LinearGrowthData,
    LinearGrowthSimSpec,
    RandomInterceptData,
    RandomInterceptSimSpec,
    generate_linear_growth,
    generate_random_intercept,
};
pub use types::{
    CovStructure,
    EstimationMethod,
    LmerConfig,
    MixedError,
    MixedModel,
    RandomTerm,
};

#[cfg(all(test, feature = "linalg"))]
mod tests {
    use std::collections::HashMap;

    use ndarray::{
        Array1,
        Array2,
        array,
    };

    use super::*;
    use crate::{
        linreg::LinearModel,
        ols,
    };

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    fn growth_cfg() -> LmerConfig {
        LmerConfig {
            max_iter: 250,
            learning_rate: 0.08,
            line_search: true,
            tol: 1e-6,
            n_restarts: 3,
            ..LmerConfig::default()
        }
    }

    #[test]
    fn empty_data_errors() {
        let y = Array1::<f64>::zeros(0);
        let x = Array2::<f64>::zeros((0, 1));
        let term = RandomTerm::random_intercept("id", Array1::zeros(0));
        let err = lmer(&y, &x, &[term], &LmerConfig::default()).unwrap_err();
        assert!(matches!(err, MixedError::EmptyData));
    }

    #[test]
    fn length_mismatch_errors() {
        let y = array![1.0, 2.0, 3.0];
        let x = array![[1.0], [2.0]];
        let term = RandomTerm::random_intercept("id", array![0, 0, 1]);
        let err = lmer(&y, &x, &[term], &LmerConfig::default()).unwrap_err();
        assert!(matches!(err, MixedError::LengthMismatch { .. }));
    }

    #[test]
    fn rejects_multiple_terms() {
        let y = array![1.0, 2.0, 3.0, 4.0];
        let x = array![[0.0], [1.0], [0.0], [1.0]];
        let t1 = RandomTerm::random_intercept("a", array![0, 0, 1, 1]);
        let t2 = RandomTerm::random_intercept("b", array![0, 1, 0, 1]);
        let err = lmer(&y, &x, &[t1, t2], &LmerConfig::default()).unwrap_err();
        assert!(matches!(err, MixedError::UnsupportedStructure { .. }));
    }

    #[test]
    fn n_le_n_fixed_errors() {
        let y = array![1.0, 2.0];
        let x = array![[1.0, 0.0], [0.0, 1.0]]; // p=2 + intercept → n_fixed=3 > n
        let term = RandomTerm::random_intercept("id", array![0, 1]);
        let err = lmer(&y, &x, &[term], &LmerConfig::default()).unwrap_err();
        assert!(matches!(err, MixedError::InvalidGroups { .. }));
    }

    #[test]
    fn sigma_u2_near_zero_matches_ols() {
        let spec = RandomInterceptSimSpec {
            n_groups: 30,
            n_per_group: 4,
            intercept: 2.0,
            coefficients: array![1.5],
            sigma2: 1.0,
            sigma_u2: 1e-10,
            seed: 7,
        };
        let data = generate_random_intercept(&spec).unwrap();
        let term = RandomTerm::random_intercept("id", data.groups.clone());
        let fit = lmer(&data.y, &data.x, &[term], &LmerConfig::default()).unwrap();
        let ols_fit = ols(&data.x, &data.y);

        assert!(
            approx(fit.fixed.intercept, ols_fit.intercept, 1e-4),
            "intercept {} vs ols {}",
            fit.fixed.intercept,
            ols_fit.intercept
        );
        assert!(
            approx(fit.fixed.coefficients[0], ols_fit.coefficients[0], 1e-4),
            "coef {} vs ols {}",
            fit.fixed.coefficients[0],
            ols_fit.coefficients[0]
        );
        let su2 = fit.sigma_u2("id").unwrap();
        assert!(su2 < 0.05, "expected tiny sigma_u2, got {su2}");
    }

    #[test]
    fn recovers_variance_components_intercept() {
        let spec = RandomInterceptSimSpec {
            n_groups: 80,
            n_per_group: 6,
            intercept: 0.5,
            coefficients: array![1.0],
            sigma2: 1.0,
            sigma_u2: 4.0,
            seed: 123,
        };
        let data = generate_random_intercept(&spec).unwrap();
        let term = RandomTerm::random_intercept("subject", data.groups.clone());
        let fit = lmer(&data.y, &data.x, &[term], &LmerConfig::default()).unwrap();

        let su2 = fit.sigma_u2("subject").unwrap();
        assert!(
            approx(fit.sigma2, spec.sigma2, 0.45),
            "sigma2 {} vs true {}",
            fit.sigma2,
            spec.sigma2
        );
        assert!(
            approx(su2, spec.sigma_u2, 1.5),
            "sigma_u2 {su2} vs true {}",
            spec.sigma_u2
        );
        assert!(fit.loglik.is_finite());
        assert_eq!(fit.re_dim["subject"], 1);
        assert_eq!(fit.ranef("subject").unwrap().ncols(), 1);
    }

    #[test]
    fn reml_vs_ml_both_finite() {
        let data = generate_random_intercept(&RandomInterceptSimSpec {
            n_groups: 25,
            n_per_group: 4,
            ..Default::default()
        })
        .unwrap();
        let term = RandomTerm::random_intercept("id", data.groups.clone());
        let reml = lmer(
            &data.y,
            &data.x,
            &[term.clone()],
            &LmerConfig {
                method: EstimationMethod::Reml,
                ..LmerConfig::default()
            },
        )
        .unwrap();
        let ml = lmer(
            &data.y,
            &data.x,
            &[term],
            &LmerConfig {
                method: EstimationMethod::Ml,
                ..LmerConfig::default()
            },
        )
        .unwrap();
        assert!(reml.loglik.is_finite() && ml.loglik.is_finite());
        // Both should recover similar fixed effects
        assert!(approx(reml.fixed.intercept, ml.fixed.intercept, 0.5));
    }

    #[test]
    fn predict_population_and_conditional_intercept() {
        let spec = RandomInterceptSimSpec {
            n_groups: 20,
            n_per_group: 5,
            intercept: 1.0,
            coefficients: array![0.0],
            sigma2: 0.25,
            sigma_u2: 1.0,
            seed: 99,
        };
        let mut data = generate_random_intercept(&spec).unwrap();
        data.x = Array2::zeros((data.y.len(), 0));
        data.true_fixed = LinearModel {
            intercept: 1.0,
            coefficients: Array1::zeros(0),
        };

        let term = RandomTerm::random_intercept("id", data.groups.clone());
        let fit = lmer(&data.y, &data.x, &[term], &LmerConfig::default()).unwrap();

        let y_pop = fit.predict(&data.x);
        assert_eq!(y_pop.len(), data.y.len());
        for v in y_pop.iter() {
            assert!(approx(*v, fit.fixed.intercept, 1e-9));
        }

        let mut gmap: HashMap<String, &[usize]> = HashMap::new();
        let gslice = data.groups.as_slice().expect("contiguous groups");
        gmap.insert("id".into(), gslice);
        let zmap: HashMap<String, &Array2<f64>> = HashMap::new();
        let y_cond = fit.predict_conditional(&data.x, &gmap, &zmap).unwrap();
        let pop_var = y_pop.iter().map(|v| (v - fit.fixed.intercept).powi(2)).sum::<f64>();
        let cond_var = y_cond.iter().map(|v| (v - fit.fixed.intercept).powi(2)).sum::<f64>();
        assert!(cond_var > pop_var + 1e-6);
        assert_eq!(fit.ranef("id").unwrap().nrows(), spec.n_groups);
    }

    #[test]
    fn arbitrary_group_labels_remapped() {
        let y = array![1.0, 1.2, 5.0, 5.1, 5.3];
        let x = Array2::<f64>::zeros((5, 0));
        let groups = array![10, 10, 99, 99, 99];
        let term = RandomTerm::random_intercept("id", groups.clone());
        let fit = lmer(&y, &x, &[term], &LmerConfig::default()).unwrap();
        assert_eq!(fit.n_groups["id"], 2);
        assert_eq!(fit.group_labels["id"], vec![10, 99]);
        assert_eq!(fit.ranef("id").unwrap().nrows(), 2);
    }

    #[test]
    fn summary_nonempty() {
        let data = generate_random_intercept(&RandomInterceptSimSpec {
            n_groups: 10,
            n_per_group: 3,
            ..Default::default()
        })
        .unwrap();
        let term = RandomTerm::random_intercept("id", data.groups.clone());
        let fit = lmer_default(&data.y, &data.x, &[term]).unwrap();
        let s = fit.summary();
        assert!(s.contains("sigma2"));
        assert!(s.contains("Linear mixed model"));
    }

    #[test]
    fn linear_growth_fits_and_predicts() {
        let data = generate_linear_growth(&LinearGrowthSimSpec::default()).unwrap();
        let term = RandomTerm::linear_growth("id", data.groups.clone(), &data.time).unwrap();
        let z = term.z_cols.clone().expect("z");

        let fit = lmer(&data.y, &data.x, &[term], &growth_cfg()).unwrap();

        assert_eq!(fit.re_dim["id"], 2);
        assert_eq!(fit.ranef("id").unwrap().shape(), &[data.true_u.nrows(), 2]);

        let g = fit.re_covariance("id").unwrap();
        assert_eq!(g.shape(), &[2, 2]);
        assert!(g[[0, 0]] > 0.0 && g[[1, 1]] > 0.0);
        assert!(fit.sigma2 > 0.0);
        assert!(fit.loglik.is_finite());

        assert!(
            approx(fit.fixed.intercept, data.true_fixed.intercept, 0.6),
            "intercept {} vs {}",
            fit.fixed.intercept,
            data.true_fixed.intercept
        );
        assert!(
            approx(fit.fixed.coefficients[0], data.true_fixed.coefficients[0], 0.35),
            "slope {} vs {}",
            fit.fixed.coefficients[0],
            data.true_fixed.coefficients[0]
        );

        assert!(
            approx(fit.sigma2, data.true_sigma2, 0.45),
            "sigma2 {} vs {}",
            fit.sigma2,
            data.true_sigma2
        );
        assert!(
            approx(g[[0, 0]], data.true_re_cov[[0, 0]], 2.5),
            "G00 {} vs {}",
            g[[0, 0]],
            data.true_re_cov[[0, 0]]
        );
        assert!(
            approx(g[[1, 1]], data.true_re_cov[[1, 1]], 0.35),
            "G11 {} vs {}",
            g[[1, 1]],
            data.true_re_cov[[1, 1]]
        );

        let y_pop = fit.predict(&data.x);
        let mut gmap: HashMap<String, &[usize]> = HashMap::new();
        gmap.insert("id".into(), data.groups.as_slice().unwrap());
        let mut zmap: HashMap<String, &Array2<f64>> = HashMap::new();
        zmap.insert("id".into(), &z);
        let y_cond = fit.predict_conditional(&data.x, &gmap, &zmap).unwrap();

        let rss_pop: f64 = data.y.iter().zip(y_pop.iter()).map(|(a, b)| (a - b).powi(2)).sum();
        let rss_cond: f64 = data.y.iter().zip(y_cond.iter()).map(|(a, b)| (a - b).powi(2)).sum();
        assert!(
            rss_cond < rss_pop * 0.95,
            "conditional RSS {rss_cond} should improve on pop {rss_pop}"
        );
    }

    #[test]
    fn linear_growth_diagonal_positive_variances() {
        let mut g = Array2::<f64>::zeros((2, 2));
        g[[0, 0]] = 3.0;
        g[[1, 1]] = 0.5;
        // zero off-diagonal — matches diagonal structure
        let spec = LinearGrowthSimSpec {
            n_groups: 50,
            n_per_group: 5,
            re_cov: g,
            seed: 21,
            ..Default::default()
        };
        let data = generate_linear_growth(&spec).unwrap();
        let term = RandomTerm::linear_growth_diagonal("id", data.groups.clone(), &data.time).unwrap();
        let fit = lmer(&data.y, &data.x, &[term], &growth_cfg()).unwrap();
        let gest = fit.re_covariance("id").unwrap();
        assert!(gest[[0, 0]] > 0.05);
        assert!(gest[[1, 1]] > 0.01);
        // Diagonal fit should keep off-diagonals at 0
        assert!(gest[[0, 1]].abs() < 1e-9, "diagonal G off-diag {}", gest[[0, 1]]);
        assert!(approx(fit.fixed.coefficients[0], spec.slope, 0.4));
    }

    #[test]
    fn linear_growth_near_zero_slope_variance() {
        let mut g = Array2::<f64>::zeros((2, 2));
        g[[0, 0]] = 4.0;
        g[[1, 1]] = 1e-10;
        let spec = LinearGrowthSimSpec {
            n_groups: 40,
            n_per_group: 5,
            re_cov: g,
            slope: 0.3,
            seed: 3,
            ..Default::default()
        };
        let data = generate_linear_growth(&spec).unwrap();
        let term = RandomTerm::linear_growth("id", data.groups.clone(), &data.time).unwrap();
        let fit = lmer(&data.y, &data.x, &[term], &growth_cfg()).unwrap();
        let gest = fit.re_covariance("id").unwrap();
        // Slope variance should stay small relative to intercept variance
        assert!(
            gest[[1, 1]] < 0.5,
            "expected small slope variance, got {}",
            gest[[1, 1]]
        );
        assert!(gest[[0, 0]] > 0.5);
        assert!(approx(fit.fixed.coefficients[0], spec.slope, 0.35));
    }

    #[test]
    fn linear_growth_unbalanced_groups() {
        let mut counts = vec![3usize; 40];
        for (i, c) in counts.iter_mut().enumerate() {
            *c = 3 + (i % 4); // 3..6
        }
        let spec = LinearGrowthSimSpec {
            n_groups: 40,
            n_per_group: 1, // ignored
            n_per: Some(counts.clone()),
            seed: 8,
            ..Default::default()
        };
        let data = generate_linear_growth(&spec).unwrap();
        assert_eq!(data.y.len(), counts.iter().sum::<usize>());

        let term = RandomTerm::linear_growth("id", data.groups.clone(), &data.time).unwrap();
        let fit = lmer(&data.y, &data.x, &[term], &growth_cfg()).unwrap();
        assert_eq!(fit.n_groups["id"], 40);
        assert!(fit.sigma2 > 0.0);
        assert!(approx(fit.fixed.intercept, spec.intercept, 0.75));
        assert!(approx(fit.fixed.coefficients[0], spec.slope, 0.4));
    }

    #[test]
    fn linear_growth_time_length_error() {
        let groups = array![0, 0, 1, 1];
        let time = array![0.0, 1.0, 0.0];
        let err = RandomTerm::linear_growth("id", groups, &time).unwrap_err();
        assert!(matches!(err, MixedError::LengthMismatch { .. }));
    }

    #[test]
    fn design_helpers_in_lmer_pipeline() {
        // Centered time + z from helpers should fit similarly to raw linear_growth
        let data = generate_linear_growth(&LinearGrowthSimSpec {
            n_groups: 45,
            n_per_group: 5,
            seed: 17,
            ..Default::default()
        })
        .unwrap();

        let (z, t_c, _origin) = z_intercept_slope_centered(&data.time, None).unwrap();
        // Fixed design: centered time only (no intercept col)
        let x = time_powers(&t_c, 1);
        let term = RandomTerm {
            name: "id".into(),
            groups: data.groups.clone(),
            z_cols: Some(z.clone()),
            cov_structure: CovStructure::Unstructured,
        };
        let fit = lmer(&data.y, &x, &[term], &growth_cfg()).unwrap();
        assert!(fit.loglik.is_finite());
        assert_eq!(fit.re_dim["id"], 2);

        // Piecewise fixed design: knots strictly inside the time range so hinges
        // are not zero columns (time grid is 0..n_per-1).
        let fx = fixed_piecewise_linear(&data.time, &[1.0, 3.0]);
        assert_eq!(fx.nrows(), data.y.len());
        assert_eq!(fx.ncols(), 3); // t + 2 hinges
        let term_ri = RandomTerm::random_intercept("id", data.groups.clone());
        let fit2 = lmer(&data.y, &fx, &[term_ri], &LmerConfig::default()).unwrap();
        assert_eq!(fit2.n_fixed, 4); // intercept + 3 fixed cols
        assert!(fit2.loglik.is_finite());
    }

    #[test]
    fn ml_estimation_method_on_growth() {
        let data = generate_linear_growth(&LinearGrowthSimSpec {
            n_groups: 40,
            n_per_group: 5,
            seed: 5,
            ..Default::default()
        })
        .unwrap();
        let term = RandomTerm::linear_growth("id", data.groups.clone(), &data.time).unwrap();
        let fit = lmer(
            &data.y,
            &data.x,
            &[term],
            &LmerConfig {
                method: EstimationMethod::Ml,
                ..growth_cfg()
            },
        )
        .unwrap();
        assert!(matches!(fit.method, EstimationMethod::Ml));
        assert!(fit.loglik.is_finite());
        assert!(fit.sigma2 > 0.0);
    }

    #[test]
    fn predict_conditional_requires_z_for_q2() {
        let data = generate_linear_growth(&LinearGrowthSimSpec {
            n_groups: 20,
            n_per_group: 4,
            seed: 2,
            ..Default::default()
        })
        .unwrap();
        let term = RandomTerm::linear_growth("id", data.groups.clone(), &data.time).unwrap();
        let fit = lmer(&data.y, &data.x, &[term], &growth_cfg()).unwrap();
        let mut gmap: HashMap<String, &[usize]> = HashMap::new();
        gmap.insert("id".into(), data.groups.as_slice().unwrap());
        let zmap: HashMap<String, &Array2<f64>> = HashMap::new();
        let err = fit.predict_conditional(&data.x, &gmap, &zmap).unwrap_err();
        assert!(matches!(err, MixedError::MissingFactor { .. }));
    }
}
