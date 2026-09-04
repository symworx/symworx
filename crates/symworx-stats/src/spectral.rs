// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Spectral summaries that consume a PSD (no FFT).
//!
//! The Welch estimator lives in `symworx-signal` (`welch` / `welch_default`).
//! This module integrates a onesided density over a frequency band.

/// Integrate `psd` over `[f_lo, f_hi]` with the trapezoid rule on `freqs`.
///
/// `freqs` must be nondecreasing. Overlapping bin edges are clipped to the
/// requested band. Length mismatch, empty input, or `f_hi <= f_lo` → `NaN`.
pub fn bandpower(freqs: &[f64], psd: &[f64], f_lo: f64, f_hi: f64) -> f64 {
    if freqs.len() != psd.len() || freqs.len() < 2 || !f_lo.is_finite() || !f_hi.is_finite() || f_hi <= f_lo {
        return f64::NAN;
    }

    let mut acc = 0.0;
    let mut any = false;
    for i in 1..freqs.len() {
        let mut x0 = freqs[i - 1];
        let mut x1 = freqs[i];
        if !x0.is_finite() || !x1.is_finite() || x1 <= x0 {
            continue;
        }
        let y0 = psd[i - 1];
        let y1 = psd[i];
        if !y0.is_finite() || !y1.is_finite() {
            continue;
        }
        if x1 <= f_lo || x0 >= f_hi {
            continue;
        }
        let mut v0 = y0;
        let mut v1 = y1;
        if x0 < f_lo {
            let t = (f_lo - x0) / (x1 - x0);
            v0 = y0 + t * (y1 - y0);
            x0 = f_lo;
        }
        if x1 > f_hi {
            let t = (f_hi - freqs[i - 1]) / (freqs[i] - freqs[i - 1]);
            v1 = psd[i - 1] + t * (psd[i] - psd[i - 1]);
            x1 = f_hi;
        }
        acc += 0.5 * (v0 + v1) * (x1 - x0);
        any = true;
    }
    if any { acc } else { f64::NAN }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bandpower_flat_psd() {
        let freqs = [0.0, 1.0, 2.0, 3.0];
        let psd = [2.0, 2.0, 2.0, 2.0];
        let p = bandpower(&freqs, &psd, 0.0, 3.0);
        assert!((p - 6.0).abs() < 1e-12, "{p}");
        let mid = bandpower(&freqs, &psd, 1.0, 2.0);
        assert!((mid - 2.0).abs() < 1e-12, "{mid}");
    }

    #[test]
    fn bandpower_bad_inputs() {
        assert!(bandpower(&[0.0], &[1.0], 0.0, 1.0).is_nan());
        assert!(bandpower(&[0.0, 1.0], &[1.0, 1.0], 1.0, 0.0).is_nan());
        assert!(bandpower(&[0.0, 1.0], &[1.0], 0.0, 1.0).is_nan());
    }
}
