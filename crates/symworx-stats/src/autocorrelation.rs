// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

/// Autocovariance at lags `0..n−1` (mean-centered).
///
/// `unbiased` divides lag-*k* by `n−k`; otherwise by `n`.
/// Lag 0 is the population second moment about the mean, not 1.
/// For a correlation-normalized curve use [`acf_normalized`].
/// For `corr(driver[t−d], target[t])` use [`ccf`].
pub fn acf(signal: &[f64], unbiased: bool) -> Vec<f64> {
    let n = signal.len();
    let mean = signal.iter().sum::<f64>() / n as f64;
    let centered: Vec<f64> = signal.iter().map(|x| x - mean).collect();

    let mut acf = Vec::with_capacity(n);

    for lag in 0..n {
        let mut sum = 0.0;
        for i in 0..(n - lag) {
            sum += centered[i] * centered[i + lag];
        }

        let norm = if unbiased { (n - lag) as f64 } else { n as f64 };

        acf.push(sum / norm);
    }

    acf
}

/// Correlation-normalized autocovariance at lags `0..=max_lag`.
///
/// Each lag uses the same divisor as [`acf`], then divides by lag 0, so
/// the first entry is `1` when the series has positive variance. Length is
/// `max_lag + 1`. Lags past `n − 1` are `0`. Returns an empty curve when
/// the series has fewer than 2 samples, a non-finite value, or near-zero
/// variance.
pub fn acf_normalized(signal: &[f64], max_lag: usize, unbiased: bool) -> Vec<f64> {
    let n = signal.len();
    if n < 2 || signal.iter().any(|v| !v.is_finite()) {
        return Vec::new();
    }
    let mean = signal.iter().sum::<f64>() / n as f64;
    let centered: Vec<f64> = signal.iter().map(|x| x - mean).collect();
    let cov = |lag: usize| -> f64 {
        let mut sum = 0.0;
        for i in 0..(n - lag) {
            sum += centered[i] * centered[i + lag];
        }
        let norm = if unbiased { (n - lag) as f64 } else { n as f64 };
        sum / norm
    };
    let c0 = cov(0);
    if !c0.is_finite() || c0.abs() < 1e-12 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(max_lag + 1);
    for lag in 0..=max_lag {
        if lag >= n {
            out.push(0.0);
        } else {
            out.push(cov(lag) / c0);
        }
    }
    out
}

/// Pearson *r* of `driver[t−d]` with `target[t]` for `d = 0..=max_lag`.
///
/// Index `d` is a backward shift of the driver, the same sign as a Kalman
/// control lag. The two series must be the same length and aligned on one
/// dense grid. Length of the result is `max_lag + 1`. A lag with fewer
/// than 2 pairs is `0`. Returns an empty curve when the lengths differ,
/// either series is shorter than 2, or any value is non-finite.
pub fn ccf(driver: &[f64], target: &[f64], max_lag: usize) -> Vec<f64> {
    let n = driver.len();
    if n != target.len() || n < 2 || driver.iter().chain(target.iter()).any(|v| !v.is_finite()) {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(max_lag + 1);
    for lag in 0..=max_lag {
        if n <= lag + 1 {
            out.push(0.0);
            continue;
        }
        out.push(pearson(&driver[..n - lag], &target[lag..]));
    }
    out
}

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len();
    if n < 2 || y.len() != n {
        return 0.0;
    }
    let mean_x = x.iter().sum::<f64>() / n as f64;
    let mean_y = y.iter().sum::<f64>() / n as f64;
    let mut num = 0.0;
    let mut sx = 0.0;
    let mut sy = 0.0;
    for i in 0..n {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        num += dx * dy;
        sx += dx * dx;
        sy += dy * dy;
    }
    let den = (sx * sy).sqrt();
    if den < 1e-12 { 0.0 } else { num / den }
}

// TESTS
#[cfg(test)]
mod test_acf {
    use super::*;

    fn lcg(n: usize, seed: u64) -> Vec<f64> {
        let mut s = seed;
        (0..n)
            .map(|_| {
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
                ((s >> 33) as f64) / (u32::MAX as f64)
            })
            .collect()
    }

    #[test]
    fn test_acf() {
        let signal = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = acf(&signal, false);
        assert_eq!(result.len(), signal.len());
        assert!(result[0] > 0.0);
    }

    #[test]
    fn normalized_matches_acf_ratio_and_is_one_at_lag_zero() {
        let signal = [1.0, 2.0, 3.0, 4.0, 5.0];
        let raw = acf(&signal, false);
        let mean = 3.0;
        let var = signal.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / 5.0;
        assert!((raw[0] - var).abs() < 1e-12);

        let norm = acf_normalized(&signal, 4, false);
        assert_eq!(norm.len(), 5);
        assert!((norm[0] - 1.0).abs() < 1e-12);
        for (k, r) in norm.iter().enumerate() {
            assert!((r - raw[k] / raw[0]).abs() < 1e-12);
        }

        let past = acf_normalized(&signal, 10, false);
        assert_eq!(past.len(), 11);
        assert!(past[5..].iter().all(|v| *v == 0.0));

        let unbiased = acf_normalized(&signal, 4, true);
        assert!((unbiased[0] - 1.0).abs() < 1e-12);
        assert!((unbiased[1] - norm[1]).abs() > 1e-12);
    }

    #[test]
    fn constant_or_non_finite_normalized_acf_is_empty() {
        assert!(acf_normalized(&[1.0, 1.0, 1.0], 4, false).is_empty());
        assert!(acf_normalized(&[1.0], 2, false).is_empty());
        assert!(acf_normalized(&[1.0, 2.0, f64::NAN], 2, true).is_empty());
    }

    #[test]
    fn sine_acf_is_zero_at_a_quarter_period() {
        let period = 40.0;
        let signal: Vec<f64> = (0..800)
            .map(|i| (2.0 * std::f64::consts::PI * i as f64 / period).sin())
            .collect();
        let r = acf_normalized(&signal, 20, false);
        // Quarter period is lag 10. The finite sum leaves a small residual
        // where the cosine term is exactly zero.
        assert!(r[9] > 0.1, "r9={}", r[9]);
        assert!(r[10].abs() < 0.02, "r10={}", r[10]);
        assert!(r[11] < -0.1, "r11={}", r[11]);
    }

    #[test]
    fn ccf_peaks_at_the_planted_driver_lag() {
        let driver = lcg(400, 3);
        let k = 4usize;
        let mut target = vec![0.0; driver.len()];
        for t in k..driver.len() {
            target[t] = driver[t - k];
        }
        let r = ccf(&driver, &target, 8);
        let peak = r
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i);
        assert_eq!(peak, Some(k));
        assert!(r[k] > 0.99, "r[{k}]={}", r[k]);
    }

    #[test]
    fn ccf_lag0_is_pearson_and_rejects_bad_input() {
        let x = [1.0, 2.0, 3.0, 4.0];
        let neg: Vec<f64> = x.iter().map(|v| -v).collect();
        let r = ccf(&x, &neg, 0);
        assert_eq!(r.len(), 1);
        assert!(r[0] < -0.99);
        assert!(ccf(&x, &[1.0, 2.0], 2).is_empty());
        assert!(ccf(&[1.0, f64::INFINITY], &[1.0, 2.0], 1).is_empty());
    }
}
