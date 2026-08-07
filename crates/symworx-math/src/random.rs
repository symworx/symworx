// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

use rand::Rng;
use rand_distr::{
    Beta,
    Distribution,
    Gamma,
    Normal,
};

/// Sampling utilities for various probability distributions.
pub mod sample {
    use super::*;

    /// Sample from a normal distribution.
    #[inline]
    pub fn normal(rng: &mut impl Rng, mean: f64, std_dev: f64) -> f64 {
        Normal::new(mean, std_dev).unwrap().sample(rng)
    }

    /// Sample from a beta distribution.
    #[inline]
    pub fn beta(rng: &mut impl Rng, alpha: f64, beta: f64) -> f64 {
        Beta::new(alpha, beta).unwrap().sample(rng)
    }

    /// Sample from a Gamma distribution.
    #[inline]
    pub fn gamma(rng: &mut impl Rng, shape: f64, rate: f64) -> f64 {
        Gamma::new(shape, rate).unwrap().sample(rng) // Note: rate parameterization
    }
}

// pub mod sample {
//     use super::*;

//     // Normal Distribution
//     /// Generate a sample from Normal(μ, σ) using Box-Muller transform.
//     #[inline]
//     pub fn normal(rng: &mut impl Rng, mean: f64, std_dev: f64) -> f64 {
//         if std_dev <= 0.0 {
//             return mean;
//         }

//         let u1: f64 = rng.r#gen::<f64>().max(1e-12);
//         let u2: f64 = rng.r#gen::<f64>();

//         let r: f64 = (-2.0 * u1.ln()).sqrt();
//         let theta = 2.0 * std::f64::consts::PI * u2;

//         mean + std_dev * r * theta.cos()
//     }

//     // Beta Distribution (to be implemented)
//     /// Generate a sample from Beta(α, β)
//     pub fn beta(_rng: &mut impl Rng, alpha: f64, beta: f64) -> f64 {
//         if alpha <= 0.0 || beta <= 0.0 {
//             return f64::NAN;
//         }
//         // We'll implement this using Gamma sampling (Johnk's method or Gamma ratio)
//         todo!("Beta sampling - coming soon")
//     }

//     // Gamma Distribution (to be implemented)
//     /// Generate a sample from Gamma(shape, rate)
//     pub fn gamma(_rng: &mut impl Rng, shape: f64, rate: f64) -> f64 {
//         if shape <= 0.0 || rate <= 0.0 {
//             return f64::NAN;
//         }
//         todo!("Gamma sampling - coming soon")
//     }
// }
