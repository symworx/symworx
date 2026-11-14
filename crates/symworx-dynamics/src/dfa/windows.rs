// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Scale windows for one DFA call.
//!
//! [`DfaConfig`] lives here because every field on it chooses which box
//! lengths are fitted. Heart rate and stride rate stay with the caller.

use super::DfaError;

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
    /// [`Self::min_repeats`]. Heart rate and stride rate are not inputs.
    /// A box longer than the series is still omitted.
    pub always_alpha2: bool,
}

impl Default for DfaConfig {
    fn default() -> Self {
        Self {
            order: 1,
            alpha_scales: vec![(4..=16).collect(), (16..=64).collect()],
            min_repeats: 4,
            always_alpha2: false,
        }
    }
}

/// Box lengths to fit, aligned with the resolved windows.
///
/// `None` is a window that was requested and then dropped. The first window
/// is never `None`: a series that cannot hold it is [`DfaError::InvalidScale`].
pub(super) fn select_windows(config: &DfaConfig, len: usize) -> Result<Vec<Option<Vec<usize>>>, DfaError> {
    if config.min_repeats == 0 {
        return Err(DfaError::InvalidScale);
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
            None if index == 0 => return Err(DfaError::InvalidScale),
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
fn split_window(scales: &[usize], order: usize, len: usize) -> Result<Option<&[usize]>, DfaError> {
    if scales.len() < 2 {
        return Err(DfaError::TooFewScales);
    }
    let min_scale = order.saturating_add(2);
    if scales.iter().any(|&scale| scale < min_scale) {
        return Err(DfaError::InvalidScale);
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
