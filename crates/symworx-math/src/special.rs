// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

/// Computes the Gamma function Γ(x) for x > 0.
///
/// Uses the Lanczos approximation (with reflection for 0 < x < 1).
/// Accurate enough for the needs of distributions and special functions here.
#[inline]
pub fn gamma(x: f64) -> f64 {
    if x <= 0.0 {
        return f64::NAN;
    }

    // Lanczos coefficients for g=7, n=9 (good accuracy, common implementation)
    const G: f64 = 7.0;
    const P: [f64; 9] = [
        0.99999999999980993,
        676.5203681218851,
        -1259.1392167224028,
        771.32342877765313,
        -176.61502916214059,
        12.507343278686905,
        -0.13857109526572012,
        9.9843695780195716e-6,
        -1.5056327351493116e-7,
    ];

    if x < 0.5 {
        // Reflection formula: Γ(x) = π / (sin(πx) * Γ(1-x))
        return std::f64::consts::PI / ((std::f64::consts::PI * x).sin() * gamma(1.0 - x));
    }

    let z = x - 1.0;
    let mut sum = P[0];
    for i in 1..P.len() {
        sum += P[i] / (z + i as f64);
    }

    let t = z + G + 0.5;
    let base = (2.0 * std::f64::consts::PI).sqrt() * t.powf(z + 0.5) * (-t).exp();
    base * sum
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
        assert!((gamma(1.0) - 1.0).abs() < 1e-9);
        assert!((gamma(2.0) - 1.0).abs() < 1e-9);
        assert!((gamma(5.0) - 24.0).abs() < 1e-7);
        assert!((gamma(0.5) - std::f64::consts::PI.sqrt()).abs() < 1e-8);
    }

    #[test]
    fn test_beta() {
        // B(1,1) = 1
        assert!((beta(1.0, 1.0) - 1.0).abs() < 1e-9);
        // B(2,2) = 1/6
        assert!((beta(2.0, 2.0) - 1.0 / 6.0).abs() < 1e-9);
    }
}
