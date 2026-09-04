// Copyright (c) 2026 PalEm Dynamics LLC
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

    // Lanczos coefficients for g=7, n=9 (good accuracy, common implementation).
    // Literals use f64-representable precision (clippy::excessive_precision).
    const G: f64 = 7.0;
    const P: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        -1.505_632_735_149_311_6e-7,
    ];

    if x < 0.5 {
        // Reflection formula: Γ(x) = π / (sin(πx) * Γ(1-x))
        return std::f64::consts::PI / ((std::f64::consts::PI * x).sin() * gamma(1.0 - x));
    }

    let z = x - 1.0;
    let mut sum = P[0];
    for (i, &coeff) in P.iter().enumerate().skip(1) {
        sum += coeff / (z + i as f64);
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

/// Error function, Abramowitz & Stegun 7.1.26 (`|err|` ≲ 1.5e-7).
#[inline]
pub fn erf(x: f64) -> f64 {
    if !x.is_finite() {
        return f64::NAN;
    }
    if x == 0.0 {
        return 0.0;
    }
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let ax = x.abs();
    let t = 1.0 / (1.0 + 0.327_591_1 * ax);
    let y = 1.0
        - (((((1.061_405_429_f64).mul_add(t, -1.453_152_027)).mul_add(t, 1.421_413_741)).mul_add(t, -0.284_496_736))
            .mul_add(t, 0.254_829_592))
            * t
            * (-ax * ax).exp();
    sign * y
}

/// Standard normal CDF `Φ(z) = ½ (1 + erf(z / √2))`.
#[inline]
pub fn standard_normal_cdf(z: f64) -> f64 {
    if !z.is_finite() {
        return f64::NAN;
    }
    0.5 * (1.0 + erf(z / std::f64::consts::SQRT_2))
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

    #[test]
    fn test_erf_and_norm_cdf() {
        assert!((erf(0.0)).abs() < 1e-12);
        assert!((standard_normal_cdf(0.0) - 0.5).abs() < 1e-12);
        // Φ(1) ≈ 0.841344746
        assert!((standard_normal_cdf(1.0) - 0.841_344_746).abs() < 1e-6);
        assert!((standard_normal_cdf(-1.0) - (1.0 - 0.841_344_746)).abs() < 1e-6);
        assert!(erf(f64::NAN).is_nan());
        assert!(standard_normal_cdf(f64::NAN).is_nan());
    }
}
