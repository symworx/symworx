// core/src/math/random.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use rand::Rng;

// ==========================================================
// SAMPLING METHODS
// ==========================================================
// Normal distribution
// ----------------------------------------------------------
pub fn normal_sample(rng: &mut rand::rngs::ThreadRng, mean: f64, std: f64) -> f64 {
    let u1: f64 = rng.r#gen::<f64>().max(1e-12);
    let u2: f64 = rng.r#gen::<f64>();

    let r = (-2.0_f64 * u1.ln()).sqrt();
    let theta = 2.0_f64 * std::f64::consts::PI * u2;

    mean + std * (r * theta.cos())
}

// pub enum SampleMethod {
//     Bernoulli,
//     Normal,
//     Beta,
//     Gamma,
//     Poisson
// }
