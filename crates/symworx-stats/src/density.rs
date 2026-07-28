// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Univariate histograms and kernel density estimates (KDE).
//!
//! Pure data transforms — no plotting. Pass any 1-D sample (including
//! residuals from [`crate::residuals`]) and get bin counts or density curves
//! for the TUI, notebooks, or export.

/// One histogram bin.
#[derive(Debug, Clone, PartialEq)]
pub struct HistBin {
    /// Left edge of the bin (inclusive except for the last bin which is closed).
    pub left: f64,
    /// Right edge of the bin.
    pub right: f64,
    /// Bin center `(left + right) / 2`.
    pub center: f64,
    /// Count of samples falling in this bin.
    pub count: u64,
}

impl HistBin {
    /// Bin width `right − left`.
    pub fn width(&self) -> f64 {
        self.right - self.left
    }
}

/// Histogram of a 1-D sample.
#[derive(Debug, Clone, PartialEq)]
pub struct Histogram {
    /// Ordered bins covering `[min, max]` of the data (with optional pad).
    pub bins: Vec<HistBin>,
    /// Minimum of the sample (before pad).
    pub data_min: f64,
    /// Maximum of the sample (before pad).
    pub data_max: f64,
    /// Number of samples used (`0` if empty / invalid).
    pub n: usize,
}

impl Histogram {
    /// Maximum bin count (at least 1 when `n > 0`).
    pub fn max_count(&self) -> u64 {
        self.bins
            .iter()
            .map(|b| b.count)
            .max()
            .unwrap_or(0)
            .max(if self.n > 0 { 1 } else { 0 })
    }

    /// Bin centers and counts as `(x, count)` pairs (for plotting / polygons).
    pub fn centers_counts(&self) -> Vec<(f64, f64)> {
        self.bins.iter().map(|b| (b.center, b.count as f64)).collect()
    }

    /// Nominal bin width (uses the first bin; empty → `NaN`).
    pub fn bin_width(&self) -> f64 {
        self.bins.first().map(|b| b.width()).unwrap_or(f64::NAN)
    }
}

/// Options for [`histogram`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HistogramConfig {
    /// Number of equal-width bins (clamped to ≥ 1).
    pub n_bins: usize,
}

impl Default for HistogramConfig {
    fn default() -> Self {
        Self { n_bins: 24 }
    }
}

/// Build an equal-width histogram of `data`.
///
/// Empty or all-non-finite input yields `n = 0` and empty `bins`.
/// Finite values only are used.
pub fn histogram(data: &[f64], config: &HistogramConfig) -> Histogram {
    let vals: Vec<f64> = data.iter().copied().filter(|v| v.is_finite()).collect();
    if vals.is_empty() {
        return Histogram {
            bins: Vec::new(),
            data_min: f64::NAN,
            data_max: f64::NAN,
            n: 0,
        };
    }
    let n_bins = config.n_bins.max(1);
    let mn = vals.iter().copied().fold(f64::INFINITY, f64::min);
    let mx = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = (mx - mn).max(1e-12);
    let width = span / n_bins as f64;

    let mut counts = vec![0u64; n_bins];
    for &v in &vals {
        let t = ((v - mn) / span * (n_bins as f64 - 1e-12)).floor() as usize;
        counts[t.min(n_bins - 1)] += 1;
    }

    let mut bins = Vec::with_capacity(n_bins);
    for (i, &c) in counts.iter().enumerate() {
        let left = mn + i as f64 * width;
        let right = left + width;
        bins.push(HistBin {
            left,
            right,
            center: (left + right) * 0.5,
            count: c,
        });
    }

    Histogram {
        bins,
        data_min: mn,
        data_max: mx,
        n: vals.len(),
    }
}

/// Convenience: histogram with default bin count (24).
pub fn histogram_default(data: &[f64]) -> Histogram {
    histogram(data, &HistogramConfig::default())
}

/// Configuration for Gaussian KDE.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KdeConfig {
    /// Kernel bandwidth. If `None`, use Silverman's rule of thumb.
    pub bandwidth: Option<f64>,
    /// Number of evaluation points on the grid (default 80).
    pub n_points: usize,
    /// Fractional padding of the data range on each side (default 0.05).
    pub pad_frac: f64,
}

impl Default for KdeConfig {
    fn default() -> Self {
        Self {
            bandwidth: None,
            n_points: 80,
            pad_frac: 0.05,
        }
    }
}

/// 1-D Gaussian kernel density estimate on a uniform grid.
#[derive(Debug, Clone, PartialEq)]
pub struct KdeEstimate {
    /// Evaluation abscissae.
    pub x: Vec<f64>,
    /// Density `f̂(x)` (integrates ~1 over the line).
    pub density: Vec<f64>,
    /// Bandwidth used.
    pub bandwidth: f64,
    /// Sample size used.
    pub n: usize,
}

impl KdeEstimate {
    /// `(x, density)` pairs for plotting.
    pub fn points(&self) -> Vec<(f64, f64)> {
        self.x.iter().zip(self.density.iter()).map(|(&x, &y)| (x, y)).collect()
    }

    /// Scale density to **expected counts per bin width** for overlay on a
    /// histogram: `y = f̂(x) · n · bin_width`.
    pub fn to_count_scale(&self, bin_width: f64) -> Vec<(f64, f64)> {
        let s = self.n as f64 * bin_width;
        self.x
            .iter()
            .zip(self.density.iter())
            .map(|(&x, &d)| (x, d * s))
            .collect()
    }

    /// Maximum density value (or 0 if empty).
    pub fn max_density(&self) -> f64 {
        self.density.iter().copied().fold(0.0_f64, f64::max)
    }
}

/// Silverman's rule of thumb bandwidth:
///
/// ```text
/// h = 1.06 · min(s, IQR/1.34) · n^{−1/5}
/// ```
///
/// Returns a positive finite `h`, or `NaN` if `data` has no finite points.
pub fn silverman_bandwidth(data: &[f64]) -> f64 {
    let vals: Vec<f64> = data.iter().copied().filter(|v| v.is_finite()).collect();
    let n = vals.len();
    if n == 0 {
        return f64::NAN;
    }
    if n == 1 {
        return 1.0;
    }
    let mean = vals.iter().sum::<f64>() / n as f64;
    let var = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
    let sd = var.sqrt().max(1e-12);

    let mut sorted = vals;
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let q = |p: f64| {
        let i = ((sorted.len() as f64 - 1.0) * p).round() as usize;
        sorted[i.min(sorted.len() - 1)]
    };
    let iqr = (q(0.75) - q(0.25)).abs().max(1e-12);
    let sigma = sd.min(iqr / 1.34).max(1e-12);
    let span = (sorted[sorted.len() - 1] - sorted[0]).abs().max(1e-12);
    (1.06 * sigma * (n as f64).powf(-0.2)).max(span * 1e-3)
}

/// Gaussian KDE of `data` on a uniform grid over the (padded) data range.
///
/// Empty / all-non-finite input → empty vectors, `n = 0`.
pub fn kde_gaussian(data: &[f64], config: &KdeConfig) -> KdeEstimate {
    let vals: Vec<f64> = data.iter().copied().filter(|v| v.is_finite()).collect();
    let n = vals.len();
    if n == 0 {
        return KdeEstimate {
            x: Vec::new(),
            density: Vec::new(),
            bandwidth: f64::NAN,
            n: 0,
        };
    }

    let mn = vals.iter().copied().fold(f64::INFINITY, f64::min);
    let mx = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = (mx - mn).max(1e-12);
    let pad = span * config.pad_frac.max(0.0);
    let x0 = mn - pad;
    let x1 = mx + pad;
    let x_span = (x1 - x0).max(1e-12);

    let h = config
        .bandwidth
        .filter(|b| b.is_finite() && *b > 0.0)
        .unwrap_or_else(|| silverman_bandwidth(&vals));
    let h = h.max(1e-12);
    let inv_h = 1.0 / h;
    let inv_sqrt_2pi = 1.0 / (2.0 * std::f64::consts::PI).sqrt();
    let n_pts = config.n_points.max(2);

    let mut x = Vec::with_capacity(n_pts);
    let mut density = Vec::with_capacity(n_pts);
    for i in 0..n_pts {
        let xi = x0 + x_span * (i as f64) / (n_pts as f64 - 1.0);
        let mut dens = 0.0;
        for &v in &vals {
            let u = (xi - v) * inv_h;
            dens += inv_sqrt_2pi * (-0.5 * u * u).exp();
        }
        dens = dens * inv_h / n as f64;
        x.push(xi);
        density.push(dens);
    }

    KdeEstimate {
        x,
        density,
        bandwidth: h,
        n,
    }
}

/// Convenience: Gaussian KDE with default grid / Silverman bandwidth.
pub fn kde_gaussian_default(data: &[f64]) -> KdeEstimate {
    kde_gaussian(data, &KdeConfig::default())
}

/// Histogram + KDE prepared for a combined plot (counts + count-scaled density).
#[derive(Debug, Clone, PartialEq)]
pub struct HistKde {
    /// Histogram of the sample.
    pub hist: Histogram,
    /// Gaussian KDE of the sample.
    pub kde: KdeEstimate,
    /// KDE evaluated as expected counts per bin width (same units as hist).
    pub kde_counts: Vec<(f64, f64)>,
}

/// Build histogram (default bins) + Gaussian KDE for overlay plots.
pub fn hist_kde(data: &[f64]) -> HistKde {
    hist_kde_with(data, &HistogramConfig::default(), &KdeConfig::default())
}

/// Build histogram + KDE with explicit configs.
pub fn hist_kde_with(data: &[f64], hist_cfg: &HistogramConfig, kde_cfg: &KdeConfig) -> HistKde {
    let hist = histogram(data, hist_cfg);
    let kde = kde_gaussian(data, kde_cfg);
    let bw = hist.bin_width();
    let kde_counts = if bw.is_finite() && bw > 0.0 {
        kde.to_count_scale(bw)
    } else {
        kde.points()
    };
    HistKde { hist, kde, kde_counts }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn histogram_counts_sum_to_n() {
        let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let h = histogram(&data, &HistogramConfig { n_bins: 10 });
        assert_eq!(h.n, 100);
        assert_eq!(h.bins.len(), 10);
        let sum: u64 = h.bins.iter().map(|b| b.count).sum();
        assert_eq!(sum, 100);
    }

    #[test]
    fn kde_positive_and_peaks_near_center() {
        // Cluster around 0
        let data: Vec<f64> = (-20..=20).map(|i| i as f64 * 0.1).collect();
        let k = kde_gaussian_default(&data);
        assert_eq!(k.x.len(), k.density.len());
        assert!(k.bandwidth > 0.0);
        let mid = k.density.len() / 2;
        let edge = 0;
        assert!(k.density[mid] > k.density[edge]);
    }

    #[test]
    fn empty_input() {
        let h = histogram_default(&[]);
        assert_eq!(h.n, 0);
        let k = kde_gaussian_default(&[]);
        assert_eq!(k.n, 0);
    }

    #[test]
    fn hist_kde_count_scale_finite() {
        let data = [1.0, 1.1, 1.2, 2.0, 2.1, 5.0];
        let hk = hist_kde(&data);
        assert!(!hk.kde_counts.is_empty());
        assert!(hk.kde_counts.iter().all(|(_, y)| y.is_finite()));
    }
}
