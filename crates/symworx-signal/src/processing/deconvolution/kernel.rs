// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Elimination kernel and causal convolution for hormone reconstruction.

/// Single-exponential elimination kernel.
///
/// `h[n] = exp(-n * dt / tau)` for `n = 0..len`. `dt` and `tau` share a time unit
/// (minutes or seconds). `tau` and `dt` must be positive.
pub fn exponential_kernel(dt: f64, tau: f64, len: usize) -> Vec<f64> {
    // NaN is not positive. `!(x > 0.0)` rejects it, and clippy forbids that spelling.
    if len == 0
        || dt.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
        || tau.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
    {
        return vec![];
    }
    (0..len).map(|i| (-(i as f64) * dt / tau).exp()).collect()
}

/// Causal convolution `c[i] = sum_{j=0..=i} kernel[i-j] * signal[j]`.
///
/// Kernel lags beyond `kernel.len()` are zero. The output has the same length as
/// `signal`.
pub fn convolve(signal: &[f64], kernel: &[f64]) -> Vec<f64> {
    let n = signal.len();
    if n == 0 || kernel.is_empty() {
        return vec![0.0; n];
    }
    let mut out = vec![0.0; n];
    for i in 0..n {
        let mut acc = 0.0;
        let j_min = i.saturating_sub(kernel.len() - 1);
        for j in j_min..=i {
            acc += kernel[i - j] * signal[j];
        }
        out[i] = acc;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exponential_kernel_starts_at_one() {
        let h = exponential_kernel(10.0, 15.0, 4);
        assert_eq!(h.len(), 4);
        assert!((h[0] - 1.0).abs() < 1e-12);
        assert!(h[1] < h[0] && h[2] < h[1]);
    }

    #[test]
    fn convolve_impulse_copies_the_kernel() {
        let signal = [1.0, 0.0, 0.0, 0.0];
        let kernel = [1.0, 0.5, 0.25];
        let c = convolve(&signal, &kernel);
        assert!((c[0] - 1.0).abs() < 1e-12);
        assert!((c[1] - 0.5).abs() < 1e-12);
        assert!((c[2] - 0.25).abs() < 1e-12);
        assert!((c[3] - 0.0).abs() < 1e-12);
    }
}
