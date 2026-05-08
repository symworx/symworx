// core/src/math/distributions.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

// ==========================================================
// Beta kernel functions
// ==========================================================

/// Unnormalized Beta kernel: `x^(a-1) * (1-x)^(b-1)`
///
/// This is the core part of the Beta(a, b) probability density function
/// on the interval (0, 1).
#[inline]
pub fn beta_kernel(x: f64, a: f64, b: f64) -> f64 {
    if !(0.0..1.0).contains(&x) || a <= 0.0 || b <= 0.0 {
        return 0.0;
    }
    x.powf(a - 1.0) * (1.0 - x).powf(b - 1.0)
}

/// Normalized Beta PDF (probability density function)
#[inline]
pub fn beta_pdf(x: f64, a: f64, b: f64) -> f64 {
    let kernel = beta_kernel(x, a, b);
    if kernel == 0.0 {
        return 0.0;
    }
    let norm = crate::math::special::beta(a, b);
    kernel / norm
}

// ==========================================================
// Gamma kernel functions
// ==========================================================

/// Unnormalized Gamma kernel for shape-rate parameterization.
///
/// Standard form: `x^(κ-1) * exp(-λ * x)` for x > 0.
#[inline]
pub fn gamma_kernel(x: f64, shape: f64, rate: f64) -> f64 {
    if x <= 0.0 || shape <= 0.0 || rate <= 0.0 {
        return 0.0;
    }
    x.powf(shape - 1.0) * (-rate * x).exp()
}

/// Normalized Gamma PDF (shape-rate parameterization)
#[inline]
pub fn gamma_pdf(x: f64, shape: f64, rate: f64) -> f64 {
    let kernel = gamma_kernel(x, shape, rate);
    if kernel == 0.0 {
        return 0.0;
    }
    let norm = rate.powf(shape) / crate::math::special::gamma(shape);
    kernel * norm
}

// ==========================================================
// TESTS
// ==========================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beta_kernel() {
        assert_eq!(beta_kernel(-0.1, 2.0, 5.0), 0.0);
        assert_eq!(beta_kernel(1.0, 2.0, 5.0), 0.0);
        assert!(beta_kernel(0.5, 2.0, 2.0) > 0.0);
    }

    #[test]
    fn test_gamma_kernel() {
        assert_eq!(gamma_kernel(-1.0, 3.0, 1.0), 0.0);
        assert!(gamma_kernel(1.0, 3.0, 1.0) > 0.0);
    }
}
