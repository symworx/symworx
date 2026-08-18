// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

/// Autocorrelation function at lags `0..n−1` (mean-centered).
///
/// `unbiased` divides lag-*k* by `n−k`; otherwise by `n`.
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

// TESTS
#[cfg(test)]
mod test_acf {
    use super::*;

    #[test]
    fn test_acf() {
        let signal = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = acf(&signal, false);
        assert_eq!(result.len(), signal.len());
        assert!(result[0] > 0.0); // ACF at lag 0 should be positive
    }
}
