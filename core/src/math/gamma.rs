// core/src/math/gamma.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

/// Normalized gamma-shaped inspiratory curve.
///   x^(kappa-1) * exp(-kappa*(x-1))
/// x must be in [0, 1]
///
/// # Arguments
///
/// # Returns
///
#[inline]
pub fn gamma_shape(x: f64, kappa: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    let a = kappa - 1.0;
    let b = -kappa * (x - 1.0);
    x.powf(a) * b.exp()
}
