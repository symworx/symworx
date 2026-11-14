// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Detrended fluctuation analysis, matching `symworx-dynamics`.
//!
//! [`profile`] mean-centers the measured series and cumulatively sums it.
//! [`fluctuation`] removes a polynomial in stacked boxes from both ends and
//! returns `F(n)`. [`scaling_exponent`] is the slope of `ln F(n)` against
//! `ln n`. [`dfa`] reports one slope per scale window.
//!
//! An empty [`DfaConfig::alpha_scales`] selects boxes `4..=16`, then
//! `16..=64`. A later window is absent when one of its boxes is longer than
//! the series, or when the series is shorter than [`DfaConfig::min_repeats`]
//! times its longest box. [`DfaConfig::always_alpha2`] keeps the second
//! window whenever its boxes fit. Heart rate and stride rate are not inputs.
//! This module does not call an accelerator.

use alloc::vec::Vec;

use crate::{
    NpuError,
    float,
};

const EMPTY: &str = "series is empty";
const NON_FINITE: &str = "series has a non-finite sample";
const INVALID_ORDER: &str = "polynomial order was zero";
const INVALID_SCALE: &str = "scale does not fit the series";
const TOO_FEW_SCALES: &str = "a window needs at least two scales";
const LENGTH_MISMATCH: &str = "scales and fluctuation differ in length";
const NON_POSITIVE: &str = "fluctuation is not positive";
const SINGULAR: &str = "fit is singular";

/// Built-in scale windows, in order: `4..=16`, then `16..=64`.
const DEFAULT_WINDOWS: &[(usize, usize)] = &[(4, 16), (16, 64)];

/// Polynomial order and one scale window per scaling exponent.
///
/// An empty [`Self::alpha_scales`] selects the built-in windows, in order:
/// `4..=16`, then `16..=64`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DfaConfig {
    /// Detrending polynomial order. `1` removes a line in each box.
    pub order: usize,
    /// One box-length list per exponent. Empty selects the built-in windows.
    pub alpha_scales: Vec<Vec<usize>>,
    /// Later windows are kept only when `longest box * min_repeats` fits in
    /// the series. `4` is the usual limit, so the built-in α2 (box 64) needs
    /// 256 samples. The first window only has to fit. Zero is rejected.
    pub min_repeats: usize,
    /// Keep the second window whenever its boxes fit, ignoring
    /// [`Self::min_repeats`]. A box longer than the series is still omitted.
    pub always_alpha2: bool,
}

impl Default for DfaConfig {
    fn default() -> Self {
        Self {
            order: 1,
            alpha_scales: alloc::vec![(4..=16).collect(), (16..=64).collect()],
            min_repeats: 4,
            always_alpha2: false,
        }
    }
}

/// One log-log fit, `ln F ≈ intercept + alpha * ln n`.
#[derive(Debug, Clone, PartialEq)]
pub struct DfaFit {
    /// Box lengths that entered this fit, sorted and unique.
    pub scales: Vec<usize>,
    /// Slope of `ln F(n)` against `ln n`.
    pub alpha: f64,
    /// Natural-log intercept of that line (the value of `ln F` at `n = 1`).
    pub intercept: f64,
}

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

/// Mean-centered cumulative profile `Y(k)`.
///
/// `Y(k)` is the running sum of `series - mean(series)`. The input is the
/// measured series. Passing a profile here integrates it again.
pub fn profile(series: &[f64]) -> Result<Vec<f64>, NpuError> {
    check_series(series)?;
    let mean = series.iter().sum::<f64>() / series.len() as f64;
    let mut acc = 0.0;
    let mut out = Vec::with_capacity(series.len());
    for &sample in series {
        acc += sample - mean;
        out.push(acc);
    }
    Ok(out)
}

/// Root-mean-square fluctuation of `profile` at one box length.
///
/// Boxes of length `scale` tile the profile from the start and again from the
/// end. In each box a polynomial of degree `order` is removed. `F` is the
/// root mean square of those residuals across both passes. A box must hold
/// at least `order + 2` samples.
pub fn fluctuation(profile: &[f64], scale: usize, order: usize) -> Result<f64, NpuError> {
    check_series(profile)?;
    check_order(order)?;
    check_scale(scale, order, profile.len())?;

    let n_seg = profile.len() / scale;
    let mut sum_sq = 0.0;
    for segment in profile.chunks_exact(scale) {
        sum_sq += residual_sum_sq(segment, order)?;
    }
    for seg in 0..n_seg {
        let start = profile.len() - (seg + 1) * scale;
        sum_sq += residual_sum_sq(&profile[start..start + scale], order)?;
    }
    let denom = denominator(n_seg, scale)?;
    Ok(float::sqrt(sum_sq / denom))
}

/// Ordinary least-squares slope and intercept of `ln F` against `ln n`.
///
/// The pair is `(alpha, intercept)`, with `ln F ≈ intercept + alpha * ln n`
/// (natural log). `F` must be positive. Identical scales have no slope.
pub fn scaling_exponent(scales: &[usize], fluctuation: &[f64]) -> Result<(f64, f64), NpuError> {
    if scales.is_empty() {
        return Err(NpuError::InvalidInput(EMPTY));
    }
    if scales.len() != fluctuation.len() {
        return Err(NpuError::InvalidInput(LENGTH_MISMATCH));
    }
    if scales.len() < 2 {
        return Err(NpuError::InvalidInput(TOO_FEW_SCALES));
    }
    if scales.contains(&0) {
        return Err(NpuError::InvalidInput(INVALID_SCALE));
    }
    if fluctuation.iter().any(|value| !value.is_finite()) {
        return Err(NpuError::InvalidInput(NON_FINITE));
    }
    if fluctuation.iter().any(|value| *value <= 0.0) {
        return Err(NpuError::InvalidInput(NON_POSITIVE));
    }

    let mut mean_x = 0.0;
    let mut mean_y = 0.0;
    let mut xs = Vec::with_capacity(scales.len());
    let mut ys = Vec::with_capacity(fluctuation.len());
    for (&scale, &value) in scales.iter().zip(fluctuation.iter()) {
        let x = float::ln(scale as f64);
        let y = float::ln(value);
        mean_x += x;
        mean_y += y;
        xs.push(x);
        ys.push(y);
    }
    let n = scales.len() as f64;
    mean_x /= n;
    mean_y /= n;

    let mut var_x = 0.0;
    let mut cov = 0.0;
    for i in 0..scales.len() {
        let dx = xs[i] - mean_x;
        var_x += dx * dx;
        cov += dx * (ys[i] - mean_y);
    }
    if var_x <= 1e-18 {
        return Err(NpuError::InvalidInput(SINGULAR));
    }
    let alpha = cov / var_x;
    let intercept = mean_y - alpha * mean_x;
    Ok((alpha, intercept))
}

/// Detrended fluctuation analysis of one series.
///
/// Pass the measured series (RR intervals, stride intervals). This function
/// builds the profile. A series that is already a profile would be integrated
/// twice.
pub fn dfa(series: &[f64], config: &DfaConfig) -> Result<DfaResult, NpuError> {
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

fn check_series(series: &[f64]) -> Result<(), NpuError> {
    if series.is_empty() {
        return Err(NpuError::InvalidInput(EMPTY));
    }
    if series.iter().any(|sample| !sample.is_finite()) {
        return Err(NpuError::InvalidInput(NON_FINITE));
    }
    Ok(())
}

fn check_order(order: usize) -> Result<(), NpuError> {
    if order == 0 {
        Err(NpuError::InvalidInput(INVALID_ORDER))
    } else {
        Ok(())
    }
}

/// A box needs one extra sample beyond the polynomial's coefficients so a
/// residual exists. DFA order 1 therefore requires `scale >= 3`.
fn check_scale(scale: usize, order: usize, len: usize) -> Result<(), NpuError> {
    if scale < order.saturating_add(2) || scale > len {
        Err(NpuError::InvalidInput(INVALID_SCALE))
    } else {
        Ok(())
    }
}

fn select_windows(config: &DfaConfig, len: usize) -> Result<Vec<Option<Vec<usize>>>, NpuError> {
    if config.min_repeats == 0 {
        return Err(NpuError::InvalidInput(INVALID_SCALE));
    }
    let windows = resolve_windows(&config.alpha_scales);
    let mut slots = Vec::with_capacity(windows.len());
    for (index, window) in windows.iter().enumerate() {
        let repeats = if index == 0 || (index == 1 && config.always_alpha2) {
            1
        } else {
            config.min_repeats
        };
        match split_window(window, config.order, len)? {
            None if index == 0 => return Err(NpuError::InvalidInput(INVALID_SCALE)),
            Some(scales) if !enough_repeats(scales, len, repeats) => slots.push(None),
            Some(scales) => slots.push(Some(scales.to_vec())),
            None => slots.push(None),
        }
    }
    Ok(slots)
}

fn resolve_windows(configured: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let windows: Vec<Vec<usize>> = if configured.is_empty() {
        DEFAULT_WINDOWS.iter().map(|&(lo, hi)| (lo..=hi).collect()).collect()
    } else {
        configured.to_vec()
    };
    windows
        .into_iter()
        .map(|mut scales| {
            scales.sort_unstable();
            scales.dedup();
            scales
        })
        .collect()
}

/// `Ok(None)` means a box is longer than `len`.
/// A box shorter than `order + 2` is an error for either window.
fn split_window(scales: &[usize], order: usize, len: usize) -> Result<Option<&[usize]>, NpuError> {
    if scales.len() < 2 {
        return Err(NpuError::InvalidInput(TOO_FEW_SCALES));
    }
    let min_scale = order.saturating_add(2);
    if scales.iter().any(|&scale| scale < min_scale) {
        return Err(NpuError::InvalidInput(INVALID_SCALE));
    }
    if scales.iter().any(|&scale| scale > len) {
        return Ok(None);
    }
    Ok(Some(scales))
}

/// `min_repeats` counts how many times the longest box must fit in the series.
fn enough_repeats(scales: &[usize], len: usize, min_repeats: usize) -> bool {
    scales
        .iter()
        .copied()
        .max()
        .is_some_and(|longest| longest.saturating_mul(min_repeats) <= len)
}

fn fit_window(curve_scales: &[usize], curve_f: &[f64], window: &[usize]) -> Result<DfaFit, NpuError> {
    let mut values = Vec::with_capacity(window.len());
    for &scale in window {
        let Some(pos) = curve_scales.iter().position(|&n| n == scale) else {
            return Err(NpuError::InvalidInput(INVALID_SCALE));
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

fn denominator(n_seg: usize, scale: usize) -> Result<f64, NpuError> {
    let denom = 2usize
        .checked_mul(n_seg)
        .and_then(|value| value.checked_mul(scale))
        .ok_or(NpuError::InvalidInput("fluctuation denominator overflows"))?;
    Ok(denom as f64)
}

fn residual_sum_sq(segment: &[f64], order: usize) -> Result<f64, NpuError> {
    if order == 1 {
        return Ok(linear_residual_sum_sq(segment));
    }
    polynomial_residual_sum_sq(segment, order)
}

/// Sum of squared residuals about the least-squares line.
///
/// The time index is `0 .. n-1`. `sum (t - mean_t)^2 = n (n^2 - 1) / 12`.
fn linear_residual_sum_sq(segment: &[f64]) -> f64 {
    let n = segment.len() as f64;
    let mean_t = (n - 1.0) / 2.0;
    let mean_y = segment.iter().sum::<f64>() / n;
    let mut cov = 0.0;
    for (i, &sample) in segment.iter().enumerate() {
        cov += (i as f64 - mean_t) * (sample - mean_y);
    }
    let sum_sq_t = n * (n * n - 1.0) / 12.0;
    let slope = cov / sum_sq_t;
    let intercept = mean_y - slope * mean_t;
    let mut sum_sq = 0.0;
    for (i, &sample) in segment.iter().enumerate() {
        let residual = sample - (intercept + slope * i as f64);
        sum_sq += residual * residual;
    }
    sum_sq
}

/// Sum of squared residuals about a degree-`order` polynomial.
///
/// The basis is centered and scaled onto `[-1, 1]`, so a high power does not
/// overflow a long box. The column space is still the polynomials of that degree.
fn polynomial_residual_sum_sq(segment: &[f64], order: usize) -> Result<f64, NpuError> {
    let n = segment.len();
    let p = order + 1;
    let half = (n as f64 - 1.0) / 2.0;
    let mut ata = alloc::vec![0.0; p * p];
    let mut aty = alloc::vec![0.0; p];
    for (i, &sample) in segment.iter().enumerate() {
        let t = (i as f64 - half) / half;
        let mut power = 1.0;
        let mut powers = alloc::vec![0.0; p];
        for term in &mut powers {
            *term = power;
            power *= t;
        }
        for row in 0..p {
            aty[row] += powers[row] * sample;
            for col in 0..p {
                ata[row * p + col] += powers[row] * powers[col];
            }
        }
    }
    let coeff = solve(ata, aty)?;
    let mut sum_sq = 0.0;
    for (i, &sample) in segment.iter().enumerate() {
        let t = (i as f64 - half) / half;
        let mut power = 1.0;
        let mut fitted = 0.0;
        for &weight in &coeff {
            fitted += weight * power;
            power *= t;
        }
        let residual = sample - fitted;
        sum_sq += residual * residual;
    }
    Ok(sum_sq)
}

/// Solve `A x = b` for a square row-major `A`. Partial pivoting.
fn solve(mut ata: Vec<f64>, mut aty: Vec<f64>) -> Result<Vec<f64>, NpuError> {
    let p = aty.len();
    let scale = ata.iter().fold(0.0_f64, |max, value| max.max(value.abs())).max(1.0);
    for col in 0..p {
        let mut pivot_row = col;
        let mut pivot_abs = ata[col * p + col].abs();
        for row in (col + 1)..p {
            let value = ata[row * p + col].abs();
            if value > pivot_abs {
                pivot_abs = value;
                pivot_row = row;
            }
        }
        if pivot_abs <= scale * 1e-12 {
            return Err(NpuError::InvalidInput(SINGULAR));
        }
        if pivot_row != col {
            for k in col..p {
                ata.swap(col * p + k, pivot_row * p + k);
            }
            aty.swap(col, pivot_row);
        }
        let pivot = ata[col * p + col];
        for row in (col + 1)..p {
            let factor = ata[row * p + col] / pivot;
            for k in col..p {
                ata[row * p + k] -= factor * ata[col * p + k];
            }
            aty[row] -= factor * aty[col];
        }
    }
    let mut coeff = alloc::vec![0.0; p];
    for row in (0..p).rev() {
        let mut value = aty[row];
        for k in (row + 1)..p {
            value -= ata[row * p + k] * coeff[k];
        }
        let diag = ata[row * p + row];
        if diag.abs() <= scale * 1e-12 {
            return Err(NpuError::InvalidInput(SINGULAR));
        }
        coeff[row] = value / diag;
    }
    Ok(coeff)
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
    fn profile_and_fluctuation_match_the_hand_values() {
        assert_eq!(profile(&[]), Err(NpuError::InvalidInput(EMPTY)));
        assert_eq!(profile(&[1.0, f64::NAN]), Err(NpuError::InvalidInput(NON_FINITE)));
        assert_eq!(profile(&[1.0, 2.0, 3.0]).unwrap(), alloc::vec![-1.0, -1.0, 0.0]);

        assert_close(fluctuation(&[0.0, 1.0, 2.0, 3.0, 4.0, 5.0], 3, 1).unwrap(), 0.0);
        assert_close(fluctuation(&[0.0, 0.0, 1.0, 1.0], 4, 1).unwrap(), (0.05_f64).sqrt());
        assert_close(fluctuation(&[0.0, 0.0, 0.0, 1.0, 1.0], 3, 1).unwrap(), 1.0 / 6.0);
        assert_eq!(
            fluctuation(&[0.0, 1.0, 2.0, 3.0], 2, 1),
            Err(NpuError::InvalidInput(INVALID_SCALE))
        );
        assert_eq!(
            fluctuation(&[0.0, 1.0, 2.0, 3.0], 4, 0),
            Err(NpuError::InvalidInput(INVALID_ORDER))
        );
    }

    #[test]
    fn polynomial_order_removes_that_degree() {
        let quadratic: Vec<f64> = (0..8)
            .map(|i| {
                let t = i as f64 - 3.5;
                t * t
            })
            .collect();
        assert!(fluctuation(&quadratic, 8, 2).unwrap() < 1e-9);
        assert!(fluctuation(&quadratic, 8, 1).unwrap() > 1e-3);
    }

    #[test]
    fn scaling_of_a_doubling_fluctuation_is_one_half() {
        let (alpha, intercept) = scaling_exponent(&[4, 16], &[2.0, 4.0]).unwrap();
        assert_close(alpha, 0.5);
        assert_close(intercept, 0.0);
        assert_eq!(
            scaling_exponent(&[4], &[2.0, 4.0]),
            Err(NpuError::InvalidInput(LENGTH_MISMATCH))
        );
        assert_eq!(
            scaling_exponent(&[4, 16], &[0.0, 4.0]),
            Err(NpuError::InvalidInput(NON_POSITIVE))
        );
    }

    #[test]
    fn default_windows_drop_alpha2_until_the_series_holds_four_repeats() {
        let config = DfaConfig::default();
        assert_eq!(config.order, 1);
        assert_eq!(config.min_repeats, 4);
        assert!(!config.always_alpha2);
        assert_eq!(
            config.alpha_scales,
            alloc::vec![(4..=16).collect::<Vec<_>>(), (16..=64).collect::<Vec<_>>()]
        );

        let short = dfa(&unit_noise(40, 1), &DfaConfig::default()).unwrap();
        assert!(short.alphas[0].is_some());
        assert!(short.alphas[1].is_none());
        assert_eq!(short.scales, (4..=16).collect::<Vec<_>>());

        let mid = dfa(&unit_noise(128, 1), &DfaConfig::default()).unwrap();
        assert!(mid.alphas[1].is_none());

        let keep = DfaConfig {
            always_alpha2: true,
            ..DfaConfig::default()
        };
        let kept = dfa(&unit_noise(128, 1), &keep).unwrap();
        assert_eq!(kept.alphas[1].as_ref().unwrap().scales, (16..=64).collect::<Vec<_>>());

        let still_short = dfa(&unit_noise(40, 1), &keep).unwrap();
        assert!(still_short.alphas[1].is_none());

        let both = dfa(&unit_noise(256, 1), &DfaConfig::default()).unwrap();
        assert!(both.alphas[0].is_some());
        assert!(both.alphas[1].is_some());
        assert_eq!(both.scales, (4..=64).collect::<Vec<_>>());

        let zero_repeats = DfaConfig {
            min_repeats: 0,
            ..DfaConfig::default()
        };
        assert_eq!(
            dfa(&unit_noise(256, 1), &zero_repeats),
            Err(NpuError::InvalidInput(INVALID_SCALE))
        );
    }

    #[test]
    fn a_middle_window_can_be_absent_while_neighbors_fit() {
        let series = unit_noise(40, 2);
        let windows = alloc::vec![alloc::vec![4, 8], alloc::vec![32, 48], alloc::vec![8, 12]];
        let result = dfa(&series, &config(1, windows)).unwrap();
        assert!(result.alphas[0].is_some());
        assert!(result.alphas[1].is_none());
        assert!(result.alphas[2].is_some());
        assert_eq!(result.scales, alloc::vec![4, 8, 12]);

        let err = dfa(&series, &config(1, alloc::vec![alloc::vec![32, 48], alloc::vec![4, 8]])).unwrap_err();
        assert_eq!(err, NpuError::InvalidInput(INVALID_SCALE));
    }

    #[test]
    fn white_noise_and_a_random_walk_land_near_one_half_and_three_halves() {
        let noise = unit_noise(2048, 0x1234_5678_9abc);
        let walk = {
            let mut acc = 0.0;
            noise
                .iter()
                .map(|sample| {
                    acc += sample;
                    acc
                })
                .collect::<Vec<_>>()
        };
        let noise_alpha = dfa(&noise, &DfaConfig::default()).unwrap().alphas[0]
            .as_ref()
            .unwrap()
            .alpha;
        let walk_alpha = dfa(&walk, &DfaConfig::default()).unwrap().alphas[0]
            .as_ref()
            .unwrap()
            .alpha;
        assert!((0.35..0.75).contains(&noise_alpha), "{noise_alpha}");
        assert!((1.15..1.85).contains(&walk_alpha), "{walk_alpha}");
    }

    #[test]
    fn a_constant_series_has_no_positive_fluctuation() {
        let err = dfa(
            &[2.0; 32],
            &config(1, alloc::vec![alloc::vec![4, 8], alloc::vec![8, 16]]),
        )
        .unwrap_err();
        assert_eq!(err, NpuError::InvalidInput(NON_POSITIVE));
    }
}
