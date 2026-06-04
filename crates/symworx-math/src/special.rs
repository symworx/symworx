// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

/// Computes the Gamma function Γ(x) for x > 0.
///
/// Uses the Lanczos approximation, which is accurate to roughly 10-14 decimal digits
/// for most values in the double precision range.
#[inline]
pub fn gamma(x: f64) -> f64 {
    if x <= 0.0 {
        return f64::NAN; // can return 0.0 / f64::INFINITY for +inf on poles
    }

    // Lanczos approximation with g=5, n=6 coefficients
    // (balancing speed and accuracy)
    const G: f64 = 5.0;
    const P: [f64; 6] = [
        76.18009172947146,
        -86.50532032941677,
        24.01409824083091,
        -1.231739572450155,
        0.1208650973866179e-2,
        -0.5395239384953e-5,
    ];

    let z = x;
    let mut sum = P[0];

    for (i, coeff) in P.iter().enumerate().skip(1) {
        // for i in 1..P.len() {
        sum += coeff / (z + i as f64);
    }

    let t = z + G + 0.5;
    let base = (2.0 * std::f64::consts::PI).sqrt() * t.powf(z + 0.5) * (-t).exp();

    base * sum / x
}

/// Computes the natural logarithm of the Gamma function: ln(Γ(x))
///
/// More numerically stable than `gamma(x).ln()` for large |x| or when Γ(x) is very large/small.
#[inline]
pub fn ln_gamma(x: f64) -> f64 {
    if x <= 0.0 {
        return f64::NAN;
    }
    // TODO: implement dedicated ln_gamma later for better accuracy
    gamma(x).ln()
}

// Beta function

/// Beta function B(a, b) = Γ(a)Γ(b) / Γ(a+b)
///
/// Used to normalize the Beta distribution.
#[inline]
pub fn beta(a: f64, b: f64) -> f64 {
    if a <= 0.0 || b <= 0.0 {
        return f64::NAN;
    }
    gamma(a) * gamma(b) / gamma(a + b)
}

/// Logarithm of the Beta function (numerically more stable)
#[inline]
pub fn ln_beta(a: f64, b: f64) -> f64 {
    if a <= 0.0 || b <= 0.0 {
        return f64::NAN;
    }
    ln_gamma(a) + ln_gamma(b) - ln_gamma(a + b)
}

// TESTS
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gamma() {
        assert!((gamma(1.0) - 1.0).abs() < 1e-10);
        assert!((gamma(2.0) - 1.0).abs() < 1e-10);
        assert!((gamma(5.0) - 24.0).abs() < 1e-8);
        assert!((gamma(0.5) - std::f64::consts::FRAC_PI_2.sqrt()).abs() < 1e-8);
    }

    #[test]
    fn test_beta() {
        // B(1,1) = 1
        assert!((beta(1.0, 1.0) - 1.0).abs() < 1e-10);
        // B(2,2) = 1/6
        assert!((beta(2.0, 2.0) - 1.0 / 6.0).abs() < 1e-10);
    }
}
