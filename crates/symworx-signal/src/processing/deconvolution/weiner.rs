// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Wiener deconvolution in the frequency domain.
//!
//! This is a robust, fast method for deconvolving signals when the
//! kernel (PSF) is known and noise is present.

use ndarray::Array1;
use rustfft::{FftPlanner, num_complex::Complex};

/// Performs Wiener deconvolution to recover an original signal.
///
/// # Arguments
/// * `observed` — Convolved / measured signal
/// * `kernel`   — Known impulse response / point spread function
/// * `snr`      — Estimated signal-to-noise ratio (higher = less regularization)
///
/// # Returns
/// Estimated original signal
pub fn wiener_deconvolution(
    observed: &[f64],
    kernel: &[f64],
    snr: f64,
) -> Vec<f64> {
    let n = observed.len();
    if n == 0 || kernel.is_empty() {
        return observed.to_vec();
    }

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n);
    let ifft = planner.plan_fft_inverse(n);

    // Pad kernel to signal length
    let mut kernel_padded = vec![0.0f64; n];
    for (i, &val) in kernel.iter().take(n).enumerate() {
        kernel_padded[i] = val;
    }

    // Convert to complex
    let mut obs_c: Vec<Complex<f64>> = observed.iter().map(|&x| Complex::new(x, 0.0)).collect();
    let mut kern_c: Vec<Complex<f64>> = kernel_padded.iter().map(|&x| Complex::new(x, 0.0)).collect();

    // Forward FFT
    fft.process(&mut obs_c);
    fft.process(&mut kern_c);

    // Wiener filter
    let noise_power = 1.0 / snr.max(1e-6);
    let mut result = vec![Complex::new(0.0, 0.0); n];

    for i in 0..n {
        let k = kern_c[i];
        let k_mag2 = k.norm_sqr();

        let filter = if k_mag2 > 1e-12 {
            k.conj() / (k_mag2 + noise_power)
        } else {
            Complex::new(0.0, 0.0)
        };

        result[i] = obs_c[i] * filter;
    }

    // Inverse FFT
    ifft.process(&mut result);

    // Return real part, normalized
    result.into_iter().map(|c| c.re / n as f64).collect()
}


// TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    fn synthetic_pulses() -> (Vec<f64>, Vec<f64>) {
        let n = 256;
        let mut signal = vec![0.0; n];

        // Add synthetic pulses
        for &peak in &[55, 130, 195] {
            for i in 0..n {
                let x = (i as f64 - peak as f64) / 9.0;
                signal[i] += (-0.5 * x * x).exp() * 5.0;
            }
        }

        // Example biexponential kernel (similar to GH clearance)
        let kernel: Vec<f64> = (0..40)
            .map(|i| {
                let t = i as f64 * 0.1;
                0.8 * (-t / 3.0).exp() + 0.2 * (-t / 18.0).exp()
            })
            .collect();

        (signal, kernel)
    }

    #[test]
    fn test_wiener_basic() {
        let (signal, kernel) = synthetic_pulses();
        let deconvolved = wiener_deconvolution(&signal, &kernel, 50.0);

        assert_eq!(deconvolved.len(), signal.len());
        assert!(deconvolved.iter().all(|&x| x.is_finite()));

        // Should recover significant pulse amplitude
        let max_orig = signal.iter().copied().fold(0.0, f64::max);
        let max_rec = deconvolved.iter().copied().fold(0.0, f64::max);
        assert!(max_rec > 0.4 * max_orig);
    }

    #[test]
    fn test_wiener_low_snr() {
        let (mut signal, kernel) = synthetic_pulses();

        // Add noise
        for x in &mut signal {
            *x += (rand::random::<f64>() - 0.5) * 0.4;
        }

        let deconvolved = wiener_deconvolution(&signal, &kernel, 8.0); // lower SNR

        assert_eq!(deconvolved.len(), signal.len());
        assert!(deconvolved.iter().all(|&x| x.is_finite()));
    }

    #[test]
    fn test_wiener_empty_input() {
        let empty: Vec<f64> = vec![];
        let kernel = vec![1.0, 0.5];

        let result = wiener_deconvolution(&empty, &kernel, 10.0);
        assert!(result.is_empty());
    }
}
