// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! BioSym signal state: peaks, tachogram, loaded series, RQA params.

use std::path::PathBuf;

/// Lightweight holder for RQA parameters (editable in Dynamics)
#[derive(Debug, Clone, Copy)]
pub struct RqaParams {
    pub m: usize,
    pub tau: usize,
    pub radius: f64,
    pub theiler: usize,
}

impl Default for RqaParams {
    fn default() -> Self {
        Self {
            m: 3,
            tau: 1,
            radius: 0.5,
            theiler: 1,
        }
    }
}

pub struct PendingColumnLoad {
    pub path: PathBuf,
    pub data: Vec<Vec<f64>>,
    pub columns: usize,
    pub headers: Option<Vec<String>>,
}

pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Tunable peak-detection parameters (Explore).
///
/// Absolute height/prominence are derived from the current series range:
/// `height = min + height_frac * range`, `prominence = prom_frac * range`.
/// Min peak spacing uses `min_interval_sec * fs` when `fs` is known.
#[derive(Clone, Debug)]
pub struct PeakDetectParams {
    /// Fraction of (max−min) above min for min peak height (0…1).
    pub height_frac: f64,
    /// Fraction of (max−min) for min prominence (0…1).
    pub prom_frac: f64,
    /// Minimum time between peaks (seconds); converted to sample distance via fs.
    pub min_interval_sec: f64,
    /// Match tolerance (samples) when scoring against known/generator peaks.
    pub match_tol: usize,
}

impl Default for PeakDetectParams {
    fn default() -> Self {
        Self::for_kind(SignalKind::Unknown)
    }
}

impl PeakDetectParams {
    pub fn for_kind(kind: SignalKind) -> Self {
        match kind {
            SignalKind::Ppg => Self {
                height_frac: 0.35,
                prom_frac: 0.10,
                min_interval_sec: 0.40, // ~150 bpm upper
                match_tol: 5,
            },
            SignalKind::Respiration => Self {
                height_frac: 0.25,
                prom_frac: 0.08,
                min_interval_sec: 1.5, // ~40 brpm upper
                match_tol: 8,
            },
            SignalKind::Stride | SignalKind::Unknown => Self {
                height_frac: 0.30,
                prom_frac: 0.08,
                min_interval_sec: 0.35,
                match_tol: 5,
            },
        }
    }

    pub const N_FIELDS: usize = 4;

    pub fn field_name(i: usize) -> &'static str {
        match i {
            0 => "height_frac",
            1 => "prom_frac",
            2 => "min_interval_sec",
            3 => "match_tol (samples)",
            _ => "?",
        }
    }

    /// Nudge selected field up/down. Returns true if a value changed.
    pub fn nudge(&mut self, field: usize, up: bool) -> bool {
        match field {
            0 => {
                let step = 0.02;
                let v = if up {
                    (self.height_frac + step).min(0.95)
                } else {
                    (self.height_frac - step).max(0.0)
                };
                if (v - self.height_frac).abs() < 1e-12 {
                    return false;
                }
                self.height_frac = v;
                true
            }
            1 => {
                let step = 0.02;
                let v = if up {
                    (self.prom_frac + step).min(0.95)
                } else {
                    (self.prom_frac - step).max(0.0)
                };
                if (v - self.prom_frac).abs() < 1e-12 {
                    return false;
                }
                self.prom_frac = v;
                true
            }
            2 => {
                let step = 0.05;
                let v = if up {
                    (self.min_interval_sec + step).min(10.0)
                } else {
                    (self.min_interval_sec - step).max(0.05)
                };
                if (v - self.min_interval_sec).abs() < 1e-12 {
                    return false;
                }
                self.min_interval_sec = v;
                true
            }
            3 => {
                if up {
                    self.match_tol = self.match_tol.saturating_add(1).min(50);
                } else {
                    if self.match_tol == 0 {
                        return false;
                    }
                    self.match_tol -= 1;
                }
                true
            }
            _ => false,
        }
    }
}

/// Kind of biosignal currently loaded (affects peak presets / labels).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SignalKind {
    #[default]
    Unknown,
    Ppg,
    Respiration,
    Stride,
}

impl SignalKind {
    pub fn label(self) -> &'static str {
        match self {
            SignalKind::Unknown => "signal",
            SignalKind::Ppg => "PPG",
            SignalKind::Respiration => "respiration",
            SignalKind::Stride => "stride",
        }
    }

    pub fn primary_peak_label(self) -> &'static str {
        match self {
            SignalKind::Ppg => "systolic (known)",
            SignalKind::Respiration => "inhalation (known)",
            _ => "known peaks",
        }
    }

    pub fn secondary_peak_label(self) -> &'static str {
        match self {
            SignalKind::Ppg => "diastolic (known)",
            SignalKind::Respiration => "exhalation (known)",
            _ => "known secondary",
        }
    }

    /// Guess kind from demo/export filename heuristics.
    pub fn from_path(path: &std::path::Path) -> Self {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if name.contains("ppg") {
            SignalKind::Ppg
        } else if name.contains("resp") || name.contains("breath") {
            SignalKind::Respiration
        } else if name.contains("stride") || name.contains("gait") || name.contains("step") {
            SignalKind::Stride
        } else {
            SignalKind::Unknown
        }
    }

    /// Default sampling rate used by our demo generators (when not stored on disk).
    pub fn default_fs(self) -> Option<f64> {
        match self {
            SignalKind::Ppg => Some(250.0),
            SignalKind::Respiration => Some(50.0),
            SignalKind::Stride | SignalKind::Unknown => None,
        }
    }
}

/// Explore chart mode: raw/processed waveform vs peak–peak interval tachogram.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExploreView {
    #[default]
    Waveform,
    Tachogram,
}

impl ExploreView {
    pub fn label(self) -> &'static str {
        match self {
            ExploreView::Waveform => "waveform",
            ExploreView::Tachogram => "tachogram",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            ExploreView::Waveform => ExploreView::Tachogram,
            ExploreView::Tachogram => ExploreView::Waveform,
        }
    }
}

/// Which peak set feeds the tachogram / interval series.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TachogramSource {
    #[default]
    Detected,
    KnownPrimary,
}

impl TachogramSource {
    pub fn label(self) -> &'static str {
        match self {
            TachogramSource::Detected => "detected",
            TachogramSource::KnownPrimary => "known primary",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            TachogramSource::Detected => TachogramSource::KnownPrimary,
            TachogramSource::KnownPrimary => TachogramSource::Detected,
        }
    }
}

/// Peak–peak intervals (tachogram) derived from a peak index series.
#[derive(Clone, Debug, Default)]
pub struct TachogramData {
    pub peak_indices: Vec<usize>,
    /// Peak times in seconds when `unit_is_sec`, else sample indices as f64.
    pub peak_times: Vec<f64>,
    /// Successive peak–peak intervals (length = peaks−1).
    pub intervals: Vec<f64>,
    /// Instantaneous rate (events per minute) from each interval.
    pub rates_per_min: Vec<f64>,
    pub source: TachogramSource,
    /// True when times/intervals are in seconds (fs was known).
    pub unit_is_sec: bool,
}

impl TachogramData {
    pub fn from_peak_indices(indices: &[usize], fs: Option<f64>, source: TachogramSource) -> Self {
        if indices.len() < 2 {
            return Self {
                peak_indices: indices.to_vec(),
                source,
                unit_is_sec: fs.is_some_and(|f| f > 0.0),
                ..Self::default()
            };
        }
        let mut idxs = indices.to_vec();
        idxs.sort_unstable();
        idxs.dedup();

        let (peak_times, unit_is_sec) = if let Some(fs) = fs.filter(|f| *f > 0.0) {
            (
                idxs.iter().map(|&i| i as f64 / fs).collect::<Vec<_>>(),
                true,
            )
        } else {
            (idxs.iter().map(|&i| i as f64).collect::<Vec<_>>(), false)
        };

        // Canonical successive differences (symworx-math via core).
        let intervals = symworx_core::successive_differences(&peak_times);
        let rates_per_min: Vec<f64> = intervals
            .iter()
            .map(|&dt| {
                if dt > 0.0 {
                    if unit_is_sec {
                        60.0 / dt
                    } else {
                        // dt in samples; rate undefined without fs — leave NaN
                        f64::NAN
                    }
                } else {
                    f64::NAN
                }
            })
            .collect();

        Self {
            peak_indices: idxs,
            peak_times,
            intervals,
            rates_per_min,
            source,
            unit_is_sec,
        }
    }

    pub fn n_intervals(&self) -> usize {
        self.intervals.len()
    }

    pub fn mean_interval(&self) -> Option<f64> {
        if self.intervals.is_empty() {
            None
        } else {
            Some(self.intervals.iter().sum::<f64>() / self.intervals.len() as f64)
        }
    }

    pub fn mean_rate(&self) -> Option<f64> {
        let valid: Vec<f64> = self
            .rates_per_min
            .iter()
            .copied()
            .filter(|r| r.is_finite())
            .collect();
        if valid.is_empty() {
            None
        } else {
            Some(valid.iter().sum::<f64>() / valid.len() as f64)
        }
    }
}

#[derive(Clone, Debug)]
pub struct LoadedSignal {
    pub original: Vec<f64>,
    pub current: Vec<f64>,
    pub name: String,
    pub n_samples: usize,
    /// Sampling rate in Hz when known (from demo generation).
    pub fs: Option<f64>,
    pub kind: SignalKind,
    /// Ground-truth peak indices from synthetic generators (biosym).
    /// For PPG: systolic; for respiration: inhalation (volume maxima).
    pub known_peaks_primary: Vec<usize>,
    /// Secondary ground-truth peaks (PPG diastolic / respiration exhalation).
    pub known_peaks_secondary: Vec<usize>,
    /// Peak indices from the last peak-detection run on `current`.
    pub detected_peaks: Vec<usize>,
    pub show_known_peaks: bool,
    pub show_detected_peaks: bool,
    /// Peak–peak interval series for tachogram view / export.
    pub tachogram: Option<TachogramData>,
    pub tachogram_source: TachogramSource,
}

impl LoadedSignal {
    pub fn new(series: Vec<f64>, name: String) -> Self {
        Self::with_meta(series, name, None, SignalKind::Unknown, vec![], vec![])
    }

    pub fn with_meta(
        series: Vec<f64>,
        name: String,
        fs: Option<f64>,
        kind: SignalKind,
        known_primary: Vec<usize>,
        known_secondary: Vec<usize>,
    ) -> Self {
        let n = series.len();
        let has_known = !known_primary.is_empty() || !known_secondary.is_empty();
        Self {
            original: series.clone(),
            current: series,
            name,
            n_samples: n,
            fs,
            kind,
            known_peaks_primary: known_primary,
            known_peaks_secondary: known_secondary,
            detected_peaks: vec![],
            show_known_peaks: has_known,
            show_detected_peaks: true,
            tachogram: None,
            tachogram_source: TachogramSource::Detected,
        }
    }

    pub fn reset(&mut self) {
        self.current = self.original.clone();
        self.n_samples = self.current.len();
        self.detected_peaks.clear();
        self.tachogram = None;
    }

    pub fn apply_processed(&mut self, new_series: Vec<f64>) {
        self.current = new_series;
        self.n_samples = self.current.len();
        // Peak detection was for the previous series; clear until re-run.
        self.detected_peaks.clear();
        self.tachogram = None;
    }

    /// Rebuild tachogram from the selected peak source (detected or known primary).
    pub fn rebuild_tachogram(&mut self) {
        let idxs: &[usize] = match self.tachogram_source {
            TachogramSource::Detected => &self.detected_peaks,
            TachogramSource::KnownPrimary => &self.known_peaks_primary,
        };
        if idxs.len() < 2 {
            self.tachogram = None;
            return;
        }
        self.tachogram = Some(TachogramData::from_peak_indices(
            idxs,
            self.fs,
            self.tachogram_source,
        ));
    }
}

#[derive(Debug, Clone, Default)]
pub struct BasicStats {
    pub mean: f64,
    pub std: f64,
    pub min: f64,
    pub max: f64,
    pub median: f64,
}

pub fn compute_basic_stats(data: &[f64]) -> BasicStats {
    if data.is_empty() {
        return BasicStats::default();
    }
    let n = data.len() as f64;
    let mean = data.iter().sum::<f64>() / n;
    let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    let std = variance.sqrt();
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let min = sorted[0];
    let max = *sorted.last().unwrap();
    let median = if sorted.len() % 2 == 1 {
        sorted[sorted.len() / 2]
    } else {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
    };
    BasicStats {
        mean,
        std,
        min,
        max,
        median,
    }
}
