// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Detrended fluctuation analysis (DFA).
//!
//! The estimator follows Peng. [`profile`] mean-centers the measured series
//! and cumulatively sums it. [`fluctuation`] removes a polynomial in each box,
//! tiling from both ends, and returns `F(n)`. [`scaling_exponent`] is the
//! slope of `ln F(n)` against `ln n`.
//!
//! [`dfa`] reports one slope per scale window on that curve. An empty
//! [`DfaConfig::alpha_scales`] selects the built-in pair: boxes `4..=16`,
//! then `16..=64`. A later window is absent when one of its boxes is longer
//! than the series, or when the series is shorter than
//! [`DfaConfig::min_repeats`] times its longest box. [`DfaConfig::always_alpha2`]
//! keeps the second window whenever its boxes fit. The first window is
//! required. [`DfaConfig::order`] is the
//! polynomial degree inside each box. Raising it recomputes `F(n)`. It does
//! not choose which window is which exponent.
//!
//! With the `gpu` feature, `dfa_on_device` asks HIP for order-1 `F(n)` and
//! fits the slope on the CPU. It does not fall back to [`fluctuation`].

#[cfg(feature = "gpu")]
mod device;
mod fluctuation;
mod profile;
mod scaling;
mod windows;

#[cfg(feature = "gpu")]
pub use device::dfa_on_device;
pub use fluctuation::fluctuation;
pub use profile::profile;
pub use scaling::{
    DfaFit,
    scaling_exponent,
};
pub use windows::DfaConfig;
use windows::select_windows;

/// Fluctuation curve and one scaling fit per requested window.
#[derive(Debug, Clone, PartialEq)]
pub struct DfaResult {
    /// Box lengths at which `F(n)` was computed, sorted and unique.
    pub scales: Vec<usize>,
    /// Root-mean-square fluctuation `F(n)`, aligned with [`Self::scales`].
    pub fluctuation: Vec<f64>,
    /// Fits aligned with the resolved windows. The first entry is present
    /// whenever [`dfa`] returns `Ok`. A later entry is `None` when one of its
    /// boxes is longer than the series, or when [`DfaConfig::min_repeats`]
    /// is not met. [`DfaConfig::always_alpha2`] waives that repeat rule for
    /// the second entry only.
    pub alphas: Vec<Option<DfaFit>>,
}

/// Error logging
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DfaError {
    /// The series, profile, or scale list was empty.
    Empty,
    /// A sample was NaN or infinite.
    NonFinite,
    /// Polynomial order was zero.
    InvalidOrder,
    /// A box was shorter than `order + 2`, or was zero.
    /// Also used when an `α1` box is longer than the series.
    InvalidScale,
    /// Fewer than two scales, so the log-log slope is undefined.
    TooFewScales,
    /// `scales` and `fluctuation` differ in length.
    LengthMismatch,
    /// `F(n)` was zero or negative, so `ln F` is undefined.
    NonPositiveFluctuation,
    /// A box detrend or the log-log line had no unique solution.
    SingularFit,
}

fn check_series(series: &[f64]) -> Result<(), DfaError> {
    if series.is_empty() {
        return Err(DfaError::Empty);
    }
    if series.iter().any(|sample| !sample.is_finite()) {
        return Err(DfaError::NonFinite);
    }
    Ok(())
}

fn check_order(order: usize) -> Result<(), DfaError> {
    if order == 0 {
        Err(DfaError::InvalidOrder)
    } else {
        Ok(())
    }
}

/// A box needs one extra sample beyond the polynomial's coefficients so a
/// residual exists. DFA order 1 therefore requires `scale >= 3`.
fn check_scale(scale: usize, order: usize, len: usize) -> Result<(), DfaError> {
    if scale < order.saturating_add(2) || scale > len {
        Err(DfaError::InvalidScale)
    } else {
        Ok(())
    }
}

fn fit_window(curve_scales: &[usize], curve_f: &[f64], window: &[usize]) -> Result<DfaFit, DfaError> {
    let mut values = Vec::with_capacity(window.len());
    for &scale in window {
        let Some(pos) = curve_scales.iter().position(|&n| n == scale) else {
            return Err(DfaError::InvalidScale);
        };
        values.push(curve_f[pos]);
    }
    let (alpha, intercept) = scaling_exponent(window, &values)?;
    Ok(DfaFit {
        scales: window.to_vec(),
        alpha,
        intercept,
    })
}

/// Detrended fluctuation analysis of one series.
///
/// Pass the measured series (RR intervals, stride intervals). This function
/// builds the profile. A series that is already a profile would be integrated
/// twice.
///
/// Each entry of [`DfaConfig::alpha_scales`] is one exponent. An empty list
/// uses the built-in windows `4..=16` and `16..=64`, returned in that order
/// in [`DfaResult::alphas`]. The first window has to fit the series. A later
/// window is `None` when one of its boxes is longer than the series, or when
/// the series is shorter than [`DfaConfig::min_repeats`] times that window's
/// longest box. The built-in long window runs to 64, so the default of 4
/// repeats keeps it only for a series of at least 256 samples.
/// [`DfaConfig::always_alpha2`] keeps that second window whenever its boxes
/// fit. The flag does not look at heart rate or stride rate, and it cannot
/// fit a box of 64 into a shorter series.
pub fn dfa(series: &[f64], config: &DfaConfig) -> Result<DfaResult, DfaError> {
    check_series(series)?;
    check_order(config.order)?;
    let slots = select_windows(config, series.len())?;

    let integrated = profile(series)?;
    let mut scales = Vec::new();
    for window in slots.iter().flatten() {
        scales.extend(window.iter().copied());
    }
    scales.sort_unstable();
    scales.dedup();

    let mut fluctuation_curve = Vec::with_capacity(scales.len());
    for &scale in &scales {
        fluctuation_curve.push(fluctuation(&integrated, scale, config.order)?);
    }

    let mut alphas = Vec::with_capacity(slots.len());
    for slot in slots {
        let fit = match slot {
            Some(window) => Some(fit_window(&scales, &fluctuation_curve, &window)?),
            None => None,
        };
        alphas.push(fit);
    }
    Ok(DfaResult {
        scales,
        fluctuation: fluctuation_curve,
        alphas,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(got: f64, expect: f64) {
        assert!((got - expect).abs() <= 1e-12, "{got} != {expect}");
    }

    fn config(order: usize, alpha_scales: Vec<Vec<usize>>) -> DfaConfig {
        DfaConfig {
            order,
            alpha_scales,
            min_repeats: 1,
            always_alpha2: false,
        }
    }

    fn unit_noise(n: usize, seed: u64) -> Vec<f64> {
        let mut state = seed;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let unit = (state >> 11) as f64 / (1u64 << 53) as f64;
            out.push(unit * 2.0 - 1.0);
        }
        out
    }

    #[test]
    fn default_config_is_dfa_order_1_with_builtin_windows() {
        let config = DfaConfig::default();
        assert_eq!(config.order, 1);
        assert_eq!(config.min_repeats, 4);
        assert!(!config.always_alpha2);
        assert_eq!(
            config.alpha_scales,
            vec![(4..=16).collect::<Vec<_>>(), (16..=64).collect::<Vec<_>>()]
        );
    }

    #[test]
    fn dfa_rejects_an_empty_non_finite_or_zero_order_series() {
        assert_eq!(dfa(&[], &DfaConfig::default()), Err(DfaError::Empty));
        assert_eq!(dfa(&[0.0, f64::NAN], &DfaConfig::default()), Err(DfaError::NonFinite));
        assert_eq!(
            dfa(&[0.0, f64::INFINITY], &DfaConfig::default()),
            Err(DfaError::NonFinite)
        );

        let series = vec![0.0; 32];
        assert_eq!(
            dfa(&series, &config(0, vec![vec![4, 8], vec![16, 24]])),
            Err(DfaError::InvalidOrder)
        );
    }

    #[test]
    fn dfa_rejects_a_short_or_illegal_alpha1_window() {
        let series = vec![1.0; 8];
        assert_eq!(dfa(&series, &DfaConfig::default()), Err(DfaError::InvalidScale));
        assert_eq!(dfa(&series, &config(1, vec![vec![4]])), Err(DfaError::TooFewScales));
        assert_eq!(
            dfa(&series, &config(1, vec![vec![2, 4], vec![4, 6]])),
            Err(DfaError::InvalidScale)
        );
        assert_eq!(
            dfa(&series, &config(1, vec![vec![0, 4], vec![4, 6]])),
            Err(DfaError::InvalidScale)
        );
    }

    #[test]
    fn alpha2_is_absent_when_a_long_box_does_not_fit() {
        let series: Vec<f64> = (0..40).map(|i| (i % 5) as f64).collect();
        let result = dfa(&series, &DfaConfig::default()).unwrap();
        assert_eq!(result.alphas.len(), 2);
        assert_eq!(result.alphas[0].as_ref().unwrap().scales, (4..=16).collect::<Vec<_>>());
        assert_eq!(result.scales, (4..=16).collect::<Vec<_>>());
        assert!(result.alphas[1].is_none());
        assert!(result.alphas[0].as_ref().unwrap().alpha.is_finite());
    }

    #[test]
    fn alpha2_is_an_error_when_its_window_is_illegal_rather_than_too_long() {
        let series: Vec<f64> = (0..40).map(|i| (i % 5) as f64).collect();
        assert_eq!(
            dfa(&series, &config(1, vec![vec![4, 8], vec![0, 16]])),
            Err(DfaError::InvalidScale)
        );
        assert_eq!(
            dfa(&series, &config(1, vec![vec![4, 8], vec![16]])),
            Err(DfaError::TooFewScales)
        );
    }

    #[test]
    fn dfa_matches_the_profile_fluctuation_and_scaling_pieces() {
        let series: Vec<f64> = (0..48).map(|i| ((i * 3) % 7) as f64).collect();
        let cfg = config(1, vec![vec![8, 4, 8, 6, 12], vec![16, 24]]);
        let result = dfa(&series, &cfg).unwrap();
        let integrated = profile(&series).unwrap();

        let alpha1 = result.alphas[0].as_ref().unwrap();
        let alpha2 = result.alphas[1].as_ref().unwrap();
        assert_eq!(alpha1.scales, vec![4, 6, 8, 12]);
        assert_eq!(alpha2.scales, vec![16, 24]);
        assert_eq!(result.scales, vec![4, 6, 8, 12, 16, 24]);
        for (&scale, &value) in result.scales.iter().zip(&result.fluctuation) {
            assert_close(value, fluctuation(&integrated, scale, 1).unwrap());
        }
        let (fit1, intercept1) = scaling_exponent(&alpha1.scales, &result.fluctuation[..4]).unwrap();
        assert_close(alpha1.alpha, fit1);
        assert_close(alpha1.intercept, intercept1);
        let (fit2, intercept2) = scaling_exponent(&alpha2.scales, &result.fluctuation[4..]).unwrap();
        assert_close(alpha2.alpha, fit2);
        assert_close(alpha2.intercept, intercept2);
    }

    #[test]
    fn a_constant_series_has_no_log_fluctuation() {
        let series = vec![5.0; 64];
        assert_eq!(
            dfa(&series, &config(1, vec![vec![4, 8, 16], vec![16, 32]])),
            Err(DfaError::NonPositiveFluctuation)
        );
    }

    #[test]
    fn a_third_window_stays_in_position_when_a_middle_window_does_not_fit() {
        let series: Vec<f64> = (0..40).map(|i| (i % 5) as f64).collect();
        let result = dfa(&series, &config(1, vec![vec![4, 8], vec![32, 48], vec![8, 12]])).unwrap();
        assert_eq!(result.alphas.len(), 3);
        assert_eq!(result.alphas[0].as_ref().unwrap().scales, vec![4, 8]);
        assert!(result.alphas[1].is_none());
        assert_eq!(result.alphas[2].as_ref().unwrap().scales, vec![8, 12]);
        assert_eq!(result.scales, vec![4, 8, 12]);

        assert_eq!(
            dfa(&series, &config(1, vec![vec![32, 48], vec![4, 8]])),
            Err(DfaError::InvalidScale)
        );
    }

    #[test]
    fn both_default_windows_run_once_a_box_of_64_fits() {
        let series: Vec<f64> = (0..256).map(|i| ((i * 5) % 9) as f64).collect();
        let result = dfa(&series, &DfaConfig::default()).unwrap();
        assert_eq!(result.scales, (4..=64).collect::<Vec<_>>());
        assert_eq!(result.alphas[0].as_ref().unwrap().scales, (4..=16).collect::<Vec<_>>());
        assert_eq!(result.alphas[1].as_ref().unwrap().scales, (16..=64).collect::<Vec<_>>());
        assert_eq!(result.fluctuation.len(), result.scales.len());
    }

    #[test]
    fn always_alpha2_keeps_the_long_window_when_the_repeat_rule_would_drop_it() {
        let series: Vec<f64> = (0..128).map(|i| ((i * 5) % 9) as f64).collect();
        let dropped = dfa(&series, &DfaConfig::default()).unwrap();
        assert!(dropped.alphas[0].is_some());
        assert!(dropped.alphas[1].is_none());

        let mut forced = DfaConfig::default();
        forced.always_alpha2 = true;
        let kept = dfa(&series, &forced).unwrap();
        assert_eq!(kept.alphas[1].as_ref().unwrap().scales, (16..=64).collect::<Vec<_>>());

        let short: Vec<f64> = (0..40).map(|i| (i % 5) as f64).collect();
        assert!(dfa(&short, &forced).unwrap().alphas[1].is_none());
        assert_eq!(
            dfa(&series, &{
                let mut config = DfaConfig::default();
                config.min_repeats = 0;
                config
            }),
            Err(DfaError::InvalidScale)
        );
    }

    #[test]
    fn white_noise_scales_near_one_half_and_a_walk_near_three_halves() {
        let noise = unit_noise(2048, 0x1234_5678_9abc);
        let white = dfa(&noise, &DfaConfig::default()).unwrap();
        let alpha1 = white.alphas[0].as_ref().unwrap().alpha;
        let alpha2 = white.alphas[1].as_ref().unwrap().alpha;
        assert!((0.35..0.75).contains(&alpha1), "alpha1 {alpha1}");
        assert!((0.35..0.75).contains(&alpha2), "alpha2 {alpha2}");

        let mut walk = Vec::with_capacity(noise.len());
        let mut acc = 0.0;
        for sample in noise {
            acc += sample;
            walk.push(acc);
        }
        let integrated = dfa(&walk, &DfaConfig::default()).unwrap();
        let alpha1 = integrated.alphas[0].as_ref().unwrap().alpha;
        let alpha2 = integrated.alphas[1].as_ref().unwrap().alpha;
        assert!((1.15..1.85).contains(&alpha1), "alpha1 {alpha1}");
        assert!((1.15..1.85).contains(&alpha2), "alpha2 {alpha2}");
    }
}
