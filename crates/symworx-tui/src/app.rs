use std::path::PathBuf;

use ratatui::widgets::ListState;
use symworx_spatialsym::{
    decision::{
        AgentDecision,
        SpaceAction,
    },
    synthetic,
    AgentTrajectories,
    PlayingDimensions,
    Point2,
    Vec2,
};

/// Top-level tabs for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Import,
    Explore,
    Dynamics,
    Spatial,
    LoadSym,
    // Home is special: when active (via workflow) we render a full landing instead of the bar+content
    Home,
}

impl Tab {
    pub fn title(self) -> &'static str {
        match self {
            Tab::Import => "Import",
            Tab::Explore => "Explore",
            Tab::Dynamics => "Dynamics",
            Tab::Spatial => "Spatial",
            Tab::LoadSym => "LoadSym",
            Tab::Home => "Home",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Tab::Import => 0,
            Tab::Explore => 1,
            Tab::Dynamics => 2,
            Tab::Spatial => 3,
            Tab::LoadSym => 4,
            Tab::Home => 0, // not used in main 4-tab bar
        }
    }
}

pub fn tab_titles() -> Vec<ratatui::text::Span<'static>> {
    vec!["1: Import", "2: Explore", "3: Dynamics", "4: Spatial"]
        .into_iter()
        .map(ratatui::text::Span::from)
        .collect()
}

/// High-level analysis workflow / path. Drives landing + context for sub-modes and tab adaptation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Workflow {
    #[default]
    Home,
    BioSym,
    SpatialSym,
    LoadSym,
}

/// Spatial sub-view (equivalent of sub-tabs inside the Spatial tab)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpatialView {
    #[default]
    Visualize,
    Generate,
    ImportData,
}

/// LoadSym internal views (selector "home" inside the LoadSym workflow)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadSymView {
    #[default]
    List,
    Workout,
    Calendar,
    Optimization,
    /// Catalog library: per-ride LOADsym metrics table.
    Metrics,
}

/// Workout analyzer data streams (FIT/CSV channels).
/// Toggle with keys `1`–`5`; open panels share height equally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum WorkoutStream {
    Power = 0,
    HeartRate = 1,
    Speed = 2,
    Cadence = 3,
    Elevation = 4,
}

impl WorkoutStream {
    pub const ALL: [WorkoutStream; 5] = [
        WorkoutStream::Power,
        WorkoutStream::HeartRate,
        WorkoutStream::Speed,
        WorkoutStream::Cadence,
        WorkoutStream::Elevation,
    ];
    pub const COUNT: usize = 5;

    pub fn from_index(i: usize) -> Option<Self> {
        match i {
            0 => Some(Self::Power),
            1 => Some(Self::HeartRate),
            2 => Some(Self::Speed),
            3 => Some(Self::Cadence),
            4 => Some(Self::Elevation),
            _ => None,
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn key_digit(self) -> char {
        match self {
            Self::Power => '1',
            Self::HeartRate => '2',
            Self::Speed => '3',
            Self::Cadence => '4',
            Self::Elevation => '5',
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Power => "power",
            Self::HeartRate => "hr",
            Self::Speed => "speed",
            Self::Cadence => "cad",
            Self::Elevation => "elev",
        }
    }

    pub fn chart_title(self) -> &'static str {
        match self {
            Self::Power => "Power (W)",
            Self::HeartRate => "Heart rate (bpm)",
            Self::Speed => "Speed (km/h)",
            Self::Cadence => "Cadence (rpm)",
            Self::Elevation => "Elevation (m)",
        }
    }

    /// Whether this channel has usable samples on the activity.
    pub fn present_on(self, act: &symworx_io::ActivityData) -> bool {
        match self {
            Self::Power => act.has_power(),
            Self::HeartRate => act.has_hr(),
            Self::Speed => act.has_speed(),
            Self::Cadence => act.has_cadence(),
            Self::Elevation => act.has_altitude(),
        }
    }

    /// Series values for charting (display units).
    pub fn series(self, act: &symworx_io::ActivityData) -> Vec<f64> {
        match self {
            Self::Power => act.power_series(),
            Self::HeartRate => act.hr_series(),
            Self::Speed => act.speed_kmh_series(),
            Self::Cadence => act.cadence_series(),
            Self::Elevation => act.altitude_series_m(),
        }
    }
}

/// One ride file for calendar daily list.
#[derive(Debug, Clone)]
pub struct CatalogRideRow {
    pub ride_date: String,
    pub source_file: String,
    pub tss: f64,
    pub duration_s: f64,
    pub np_w: Option<f64>,
}

/// One activity row for Metrics / Library table (from catalog).
#[derive(Debug, Clone)]
pub struct ActivityMetricsUiRow {
    pub id: i64,
    pub ride_date: String,
    pub source_file: String,
    pub duration_s: f64,
    pub sport: Option<String>,
    pub avg_power_w: Option<f64>,
    pub max_power_w: Option<f64>,
    pub np_w: Option<f64>,
    pub intensity_factor: Option<f64>,
    pub tss: Option<f64>,
    pub total_work_kj: Option<f64>,
    pub avg_hr_bpm: Option<f64>,
    pub max_hr_bpm: Option<f64>,
    pub ftp_used_w: Option<f64>,
}

/// Numeric fields plottable on Metrics trends / bi-plots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricsField {
    Tss,
    Np,
    AvgPower,
    DurationMin,
    AvgHr,
    If,
    WorkKj,
    MaxPower,
}

impl MetricsField {
    pub const ALL: [MetricsField; 8] = [
        MetricsField::Tss,
        MetricsField::Np,
        MetricsField::AvgPower,
        MetricsField::DurationMin,
        MetricsField::AvgHr,
        MetricsField::If,
        MetricsField::WorkKj,
        MetricsField::MaxPower,
    ];

    /// Short label (status / compact UI).
    pub fn label(self) -> &'static str {
        match self {
            Self::Tss => "TSLi",
            Self::Np => "SEPi",
            Self::AvgPower => "avg W",
            Self::DurationMin => "dur min",
            Self::AvgHr => "avg HR",
            Self::If => "SRIi",
            Self::WorkKj => "work kJ",
            Self::MaxPower => "max W",
        }
    }

    /// Axis / chart title: full name with LOADsym acronym when applicable.
    pub fn axis_label(self) -> &'static str {
        match self {
            Self::Tss => "Training stress (TSLi)",
            Self::Np => "Normalized power (SEPi)",
            Self::AvgPower => "Average power (W)",
            Self::DurationMin => "Duration (min)",
            Self::AvgHr => "Average heart rate (bpm)",
            Self::If => "Intensity factor (SRIi)",
            Self::WorkKj => "Total work (kJ)",
            Self::MaxPower => "Max power (W)",
        }
    }

    pub fn short_key(self) -> char {
        match self {
            Self::Tss => '1',
            Self::Np => '2',
            Self::AvgPower => '3',
            Self::DurationMin => '4',
            Self::AvgHr => '5',
            Self::If => '6',
            Self::WorkKj => '7',
            Self::MaxPower => '8',
        }
    }

    pub fn from_digit(d: char) -> Option<Self> {
        match d {
            '1' => Some(Self::Tss),
            '2' => Some(Self::Np),
            '3' => Some(Self::AvgPower),
            '4' => Some(Self::DurationMin),
            '5' => Some(Self::AvgHr),
            '6' => Some(Self::If),
            '7' => Some(Self::WorkKj),
            '8' => Some(Self::MaxPower),
            _ => None,
        }
    }

    pub fn next(self) -> Self {
        let all = Self::ALL;
        let i = all.iter().position(|&f| f == self).unwrap_or(0);
        all[(i + 1) % all.len()]
    }

    pub fn value(self, row: &ActivityMetricsUiRow) -> Option<f64> {
        let v = match self {
            Self::Tss => row.tss,
            Self::Np => row.np_w,
            Self::AvgPower => row.avg_power_w,
            Self::DurationMin => Some(row.duration_s / 60.0),
            Self::AvgHr => row.avg_hr_bpm,
            Self::If => row.intensity_factor,
            Self::WorkKj => row.total_work_kj,
            Self::MaxPower => row.max_power_w,
        }?;
        if !v.is_finite() {
            return None;
        }
        // LOADsym metrics are non-negative for plotting; floor noise / bad signs.
        Some(v.max(0.0))
    }
}

/// Chart mode inside Metrics view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MetricsChartMode {
    #[default]
    Trend,
    Biplot,
}

/// One ISO-ish week aggregate for calendar weekly list.
#[derive(Debug, Clone)]
pub struct WeeklyLoadRow {
    /// Monday of the week (`YYYY-MM-DD`)
    pub week_start: String,
    pub total_tss: f64,
    pub ride_count: i64,
    pub day_count: usize,
    /// Inclusive indices into `daily_loads` covered by this week
    pub day_index_lo: usize,
    pub day_index_hi: usize,
}

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

pub struct App {
    pub current_tab: Tab,
    pub file_list: Vec<PathBuf>,
    pub list_state: ListState,
    pub manual_path: String,
    pub status: String,
    pub loaded_signal: Option<LoadedSignal>,
    pub pending_load: Option<PendingColumnLoad>,
    pub spatial_batch: Option<AgentTrajectories>,
    pub spatial_focal: Option<Vec<Point2>>,
    pub spatial_frame_idx: usize,
    pub spatial_labels: Option<Vec<Vec<SpaceAction>>>,
    pub spatial_decisions: Option<Vec<Vec<AgentDecision>>>,
    pub spatial_events: Vec<(usize, String)>,
    pub file_filter: String,
    pub filter_mode: bool,
    pub pending_generate: bool,
    pub pending_process: bool,
    pub process_selection: usize,
    pub process_window: usize,
    /// Peak-parameter editor (Explore). Adjusting fields re-runs detection live.
    pub pending_peak_params: bool,
    pub peak_params: PeakDetectParams,
    pub peak_param_selection: usize,
    pub help_mode: bool,
    /// First Esc at a root screen arms quit; second Esc exits (also Ctrl+Q).
    pub esc_quit_pending: bool,
    // New workflow / path support
    pub current_workflow: Workflow,
    pub spatial_view: SpatialView,
    pub rqa_params: RqaParams,
    pub pending_rqa: bool,
    pub last_rqa: Option<symworx_dynamics::RqaResult>,
    /// Last computed auto-recurrence plot (for improved viz + export).
    pub last_rp: Option<symworx_dynamics::RecurrencePlot>,
    /// Optional reference series for cross-recurrence (cRQA). Set via 'p' (pin) in Explore/Dynamics.
    pub reference_series: Option<(String, Vec<f64>)>,
    /// Last cRQA result (if computed).
    pub last_crqa: Option<symworx_dynamics::RqaResult>,
    pub home_selection: usize,
    // Spatial import state (parallel to BioSym pending_load / filter)
    pub pending_spatial_import: bool,
    pub spatial_file_filter: String,

    // LoadSym state (TUI interface per approved plan + user priority)
    pub loadsym_view: LoadSymView,
    pub loadsym_selection: usize,
    /// Daily TSS (or demo loads), oldest → newest.
    pub daily_loads: Vec<f64>,
    /// Parallel dates (`YYYY-MM-DD`) when loaded from catalog; empty for synthetic demo.
    pub daily_load_dates: Vec<String>,
    /// Optional ACWR per day from `load_metrics` (same length as `daily_loads` when present).
    pub daily_acwr: Vec<Option<f64>>,
    pub daily_risk: Vec<Option<String>>,
    pub daily_ride_counts: Vec<i64>,
    /// Focus day index (oldest → newest) for calendar daily list.
    pub loadsym_scroll: usize,
    /// Focus week index for calendar weekly list (kept in sync with daily).
    pub loadsym_week_scroll: usize,
    /// When true, last scroll action was on the weekly pane (affects linked top alignment).
    pub loadsym_scroll_from_week: bool,
    /// Per-ride rows from catalog (for daily file list).
    pub catalog_rides: Vec<CatalogRideRow>,
    /// Full activity metrics for Metrics table (oldest → newest in load; UI may reverse).
    pub catalog_activity_metrics: Vec<ActivityMetricsUiRow>,
    /// Focused row index into `catalog_activity_metrics` (0 = oldest).
    pub metrics_scroll: usize,
    /// Trend vs bi-plot under the Metrics table.
    pub metrics_chart_mode: MetricsChartMode,
    /// Y metric for trend chart (vs ride index / time).
    pub metrics_trend_field: MetricsField,
    /// Bi-plot X axis field.
    pub metrics_biplot_x: MetricsField,
    /// Bi-plot Y axis field.
    pub metrics_biplot_y: MetricsField,
    /// Weekly aggregates derived from daily series (oldest → newest).
    pub weekly_loads: Vec<WeeklyLoadRow>,
    /// Path of catalog last loaded into calendar (status only; may be None).
    pub loadsym_catalog_path: Option<PathBuf>,
    /// True when daily_loads came from SQLite catalog (not synthetic `g`).
    pub loadsym_from_catalog: bool,
    /// Goal for Programming Optimization multi-day plan.
    pub loadsym_plan_goal: symworx_loadsym::load::LoadGoal,
    /// Plan horizon in days (2–[`MAX_HORIZON_DAYS`]; default 4). Select before recompute.
    pub loadsym_plan_horizon: usize,
    /// Cached plan (recomputed only when inputs change — not every frame).
    pub loadsym_cached_plan: Option<symworx_loadsym::load::LoadPlan>,
    /// Cached plan error message when optimize fails.
    pub loadsym_cached_plan_err: Option<String>,
    /// Fingerprint of (goal, horizon, loads) used for the cache.
    pub loadsym_plan_cache_key: u64,
    /// True when the user explicitly set goal with 1/2/3 (do not re-suggest until re-enter).
    pub loadsym_goal_user_override: bool,
    /// Last auto-suggestion summary for Optimization banner (empty if none).
    pub loadsym_goal_suggest_note: String,

    // Loaded activity (from .fit or activity CSV) — used by LoadSym Workout
    pub loaded_activity: Option<symworx_io::ActivityData>,
    pub activity_scroll: usize,
    /// Focused stream for summary/thresh stats (`WorkoutStream` index).
    pub activity_series: usize,
    /// Which workout streams are shown as chart panels (`WorkoutStream::COUNT`).
    /// Closed panels redistribute height among remaining open ones.
    pub workout_stream_on: [bool; WorkoutStream::COUNT],
    // Scrolling for BioSym Explore tab (long signals / tachogram x-axis)
    pub explore_scroll: usize,
    /// Waveform vs tachogram (interval) chart.
    pub explore_view: ExploreView,
    // User exploration for LoadSym Workout: custom threshold + min duration (samples)
    pub workout_user_thresh: f64,
    pub workout_user_min_dur: usize,

    // LoadSym cycling power: FTP for TSS/NP/IF calculations (W)
    pub ftp: f64,
    // Directories to scan for .fit / activity files (in addition to ./data)
    pub loadsym_archive_dirs: Vec<PathBuf>,

    /// Workout file-open modal active.
    pub pending_workout_open: bool,
    /// Discovered activity files for the open modal (newest first).
    pub workout_file_list: Vec<PathBuf>,
    /// Selection index into `workout_file_list`.
    pub workout_file_sel: usize,
    /// Selected ride index within the focused calendar day (`rides_for_focus_day`).
    pub calendar_ride_sel: usize,

    /// Live host stream (simulator / future serial). When set, Explore shows live UI.
    pub live: Option<crate::live::LiveSession>,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            current_tab: Tab::Home,
            file_list: Vec::new(),
            list_state: ListState::default(),
            manual_path: String::new(),
            status: "Home — Select path: 1=BioSym  2=LoadSym  3=SpatialSym  • Ctrl+H home"
                .to_string(),
            loaded_signal: None,
            pending_load: None,
            file_filter: String::new(),
            filter_mode: false,
            pending_generate: false,
            pending_process: false,
            process_selection: 0,
            process_window: 5,
            pending_peak_params: false,
            peak_params: PeakDetectParams::default(),
            peak_param_selection: 0,
            spatial_batch: None,
            spatial_focal: None,
            spatial_frame_idx: 0,
            spatial_labels: None,
            spatial_decisions: None,
            spatial_events: vec![],
            help_mode: false,
            esc_quit_pending: false,
            // workflow defaults
            current_workflow: Workflow::Home,
            spatial_view: SpatialView::Visualize,
            rqa_params: RqaParams::default(),
            pending_rqa: false,
            last_rqa: None,
            last_rp: None,
            reference_series: None,
            last_crqa: None,
            home_selection: 0,
            pending_spatial_import: false,
            spatial_file_filter: String::new(),
            // LoadSym defaults
            loadsym_view: LoadSymView::List,
            loadsym_selection: 0,
            daily_loads: vec![],
            daily_load_dates: vec![],
            daily_acwr: vec![],
            daily_risk: vec![],
            daily_ride_counts: vec![],
            loadsym_scroll: 0,
            loadsym_week_scroll: 0,
            loadsym_scroll_from_week: false,
            catalog_rides: vec![],
            catalog_activity_metrics: vec![],
            metrics_scroll: 0,
            metrics_chart_mode: MetricsChartMode::Trend,
            metrics_trend_field: MetricsField::Tss,
            metrics_biplot_x: MetricsField::DurationMin,
            metrics_biplot_y: MetricsField::Tss,
            weekly_loads: vec![],
            loadsym_catalog_path: None,
            loadsym_from_catalog: false,
            loadsym_plan_goal: symworx_loadsym::load::LoadGoal::Recovery,
            loadsym_plan_horizon: 4,
            loadsym_cached_plan: None,
            loadsym_cached_plan_err: None,
            loadsym_plan_cache_key: 0,
            loadsym_goal_user_override: false,
            loadsym_goal_suggest_note: String::new(),
            loaded_activity: None,
            activity_scroll: 0,
            activity_series: 0,
            workout_stream_on: [true, true, true, false, false],
            explore_scroll: 0,
            explore_view: ExploreView::Waveform,
            workout_user_thresh: 0.0,
            workout_user_min_dur: 3,
            ftp: 300.0,
            // Prefer personal velofit archive, then project-relative folders.
            loadsym_archive_dirs: symworx_io::default_activity_search_dirs(),
            pending_workout_open: false,
            workout_file_list: Vec::new(),
            workout_file_sel: 0,
            calendar_ride_sel: 0,
            live: None,
        };
        app.refresh_file_list();
        if !app.file_list.is_empty() {
            app.list_state.select(Some(0));
        }
        // Best-effort: load personal catalog for LoadSym calendar if present.
        let _ = crate::processing::try_load_loadsym_catalog(&mut app);
        // Note: synthetic data is NOT loaded by default for any workflow.
        // BioSym, LoadSym, and SpatialSym each provide explicit Generate and Import options.
        // See seed_spatial_demo(), generate_demo_and_load(), and 'g'/'i' handlers.
        app
    }

    /// Clear double-Esc quit arming (any successful Esc cancel or non-Esc key).
    pub fn clear_esc_quit(&mut self) {
        self.esc_quit_pending = false;
    }

    /// At a root screen with nothing left to cancel: first Esc arms, second quits.
    /// Returns `true` when the process should exit.
    pub fn esc_root_or_quit(&mut self) -> bool {
        if self.esc_quit_pending {
            self.esc_quit_pending = false;
            true
        } else {
            self.esc_quit_pending = true;
            self.status = "Esc again to quit  ·  Ctrl+Q also quits".to_string();
            false
        }
    }

    pub fn clear_submodes(&mut self) {
        self.help_mode = false;
        self.esc_quit_pending = false;
        self.pending_generate = false;
        self.filter_mode = false;
        self.pending_process = false;
        self.pending_peak_params = false;
        self.pending_rqa = false;
        self.pending_spatial_import = false;
        self.last_rp = None;
        self.last_crqa = None;
        // keep reference_series unless we decide to clear; for now keep across simple cancels
        self.manual_path.clear();
        self.file_filter.clear();
        self.spatial_file_filter.clear();
        // keep loadsym data but reset to list view for clean nav
        self.loadsym_view = LoadSymView::List;
        self.loaded_activity = None;
        self.activity_scroll = 0;
        self.activity_series = 0;
        self.workout_stream_on = [true, true, true, false, false];
        self.explore_scroll = 0;
        self.workout_user_thresh = 0.0;
        self.workout_user_min_dur = 3;
        self.pending_workout_open = false;
        self.workout_file_list.clear();
        self.workout_file_sel = 0;
        self.calendar_ride_sel = 0;
        self.loadsym_goal_user_override = false;
        self.loadsym_goal_suggest_note.clear();
        // Live is a modal stream — stop when clearing modes / switching home.
        self.stop_live();
    }

    /// Drain live samples into the ring buffer (call every frame).
    pub fn poll_live(&mut self) {
        if let Some(live) = self.live.as_mut() {
            live.poll();
        }
    }

    /// Whether a live session is active.
    pub fn is_live(&self) -> bool {
        self.live.is_some()
    }

    /// Start (or restart) the synthetic live stream and jump to Explore.
    pub fn start_live_simulator(&mut self) {
        self.stop_live();
        self.pending_process = false;
        self.pending_peak_params = false;
        self.help_mode = false;
        let sid = "S001".to_string();
        self.live = Some(crate::live::LiveSession::start_simulator(sid.clone()));
        self.current_workflow = Workflow::BioSym;
        self.current_tab = Tab::Explore;
        self.explore_view = ExploreView::Waveform;
        self.status = format!("LIVE · simulator · sid={} · Esc stop · Ctrl+L restart", sid);
    }

    /// Stop the live session if any (idempotent). Does not change status.
    pub fn stop_live(&mut self) {
        if let Some(session) = self.live.take() {
            session.stop();
        }
    }

    /// User-initiated stop (Esc / restart path messaging).
    pub fn stop_live_user(&mut self) {
        if self.live.is_some() {
            self.stop_live();
            self.status = "Live stream stopped.  Ctrl+L to start simulator again.".to_string();
        }
    }

    pub fn refresh_file_list(&mut self) {
        self.file_list.clear();
        self.file_filter.clear();
        let candidates = ["./data", "."];
        for dir in candidates {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            if matches!(
                                ext.to_lowercase().as_str(),
                                "csv" | "txt" | "dat" | "bin" | "biosym"
                            ) {
                                self.file_list.push(path);
                            }
                        }
                    }
                }
            }
        }
        if !self.file_list.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }

    pub fn seed_spatial_demo(&mut self) {
        use symworx_spatialsym::Point2;
        let init = vec![
            Point2::new(0., 0.),
            Point2::new(1.2, 2.5),
            Point2::new(0.7, -0.5),
        ];
        let evs = vec![
            synthetic::SpatialEvent::StartRun {
                agent: 1,
                target: Point2::new(6.3, 2.5),
                speed: 4.0,
                start_time: 0.2,
            },
            synthetic::SpatialEvent::Pass {
                from: 0,
                to: 1,
                time: 0.6,
            },
            synthetic::SpatialEvent::Close {
                agent: 2,
                target: 1,
                speed: 5.2,
                start_time: 0.7,
            },
        ];
        let (ev_t, ev_p, ev_f) =
            synthetic::generate_event_driven(init, Point2::new(0.25, 0.), &evs, 1.4, 0.1);
        let groups = vec![0u32, 0, 1];
        let att = vec![Vec2::new(1., 0.), Vec2::new(1., 0.), Vec2::new(-1., 0.)];
        let dims = Some(PlayingDimensions::new(105.0, 68.0));
        let goal_pos = vec![
            Point2::new(52.5, 0.0),
            Point2::new(52.5, 0.0),
            Point2::new(-52.5, 0.0),
        ];
        let (batch, focal) = symworx_spatialsym::build_agent_trajectories(
            ev_t.clone(),
            ev_p,
            groups,
            att,
            ev_f.clone(),
            dims,
            Some(goal_pos),
        );
        let n_steps = batch.num_times();
        self.spatial_batch = Some(batch);
        self.spatial_focal = Some(focal);
        self.spatial_frame_idx = 0;
        self.spatial_labels = Some(symworx_spatialsym::synthetic::generate_ground_truth(
            3,
            n_steps,
            "pass_then_press",
        ));
        // Wire classifier decisions so conf / features (spd, fwd, near, dfoc, etc) are populated
        if let (Some(b), Some(foc)) = (&self.spatial_batch, &self.spatial_focal) {
            let decs = b.classify_with_focal_and_params(foc, 0.5, 10.0, 0.8);
            self.spatial_decisions = Some(decs);
        }
        // Seed a few event markers for < > / digit nav (based on synthetic event times ~0.2/0.6/0.7s @dt=0.1)
        self.spatial_events = vec![
            (0, "start".to_string()),
            (2, "run".to_string()),
            (6, "pass".to_string()),
            (7, "close".to_string()),
        ];
    }

    pub fn visible_indices(&self) -> Vec<usize> {
        let needle = self.file_filter.to_lowercase();
        if needle.is_empty() {
            (0..self.file_list.len()).collect()
        } else {
            self.file_list
                .iter()
                .enumerate()
                .filter(|(_, p)| p.to_string_lossy().to_lowercase().contains(&needle))
                .map(|(i, _)| i)
                .collect()
        }
    }

    pub fn ensure_valid_selection(&mut self) {
        let vis = self.visible_indices();
        if vis.is_empty() {
            self.list_state.select(None);
            return;
        }
        if let Some(i) = self.list_state.selected() {
            if i >= vis.len() {
                self.list_state.select(Some(0));
            }
        } else {
            self.list_state.select(Some(0));
        }
    }

    pub fn select_next(&mut self) {
        let vis = self.visible_indices();
        if vis.is_empty() {
            return;
        }
        let mut i = self.list_state.selected().unwrap_or(0);
        if i >= vis.len() {
            i = 0;
        }
        i = (i + 1) % vis.len();
        self.list_state.select(Some(i));
    }

    pub fn select_prev(&mut self) {
        let vis = self.visible_indices();
        if vis.is_empty() {
            return;
        }
        let mut i = self.list_state.selected().unwrap_or(0);
        if i >= vis.len() {
            i = 0;
        }
        i = if i == 0 { vis.len() - 1 } else { i - 1 };
        self.list_state.select(Some(i));
    }

    pub fn selected_path(&self) -> Option<&PathBuf> {
        let vis = self.visible_indices();
        let pos = self.list_state.selected()?;
        let orig = *vis.get(pos)?;
        self.file_list.get(orig)
    }

    pub fn load_selected_or_manual(&mut self) -> anyhow::Result<()> {
        if let Some(path) = self.selected_path().cloned() {
            self.load_file(&path)
        } else if !self.manual_path.is_empty() {
            let path = PathBuf::from(&self.manual_path);
            self.load_file(&path)
        } else {
            anyhow::bail!("no file selected and no manual path")
        }
    }

    pub fn load_file(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        if !path.exists() {
            anyhow::bail!("file does not exist: {}", path.display());
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext == "csv" || ext == "txt" || ext == "dat" {
            return self.load_csv(path);
        }
        if ext == "ibi" {
            return self.load_ibi(path);
        }
        if let Ok(signal) = self.try_load_parquet(path) {
            self.loaded_signal = Some(signal);
            self.current_tab = Tab::Explore;
            self.current_workflow = Workflow::BioSym;
            self.status = format!("Loaded {} (switched to Explore)", path.display());
            self.ensure_status_for_current_tab();
            return Ok(());
        }
        anyhow::bail!("unsupported or failed: {}", path.display())
    }

    pub fn try_load_parquet(&self, _path: &PathBuf) -> anyhow::Result<LoadedSignal> {
        // stub, use simple or assume
        Err(anyhow::anyhow!("parquet not fully wired here"))
    }

    pub fn load_csv(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        use std::{
            fs::File,
            io::{
                BufRead,
                BufReader,
            },
        };
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut series = Vec::new();
        let mut has_header = false;
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if !has_header {
                if trimmed.contains(',') || trimmed.parse::<f64>().is_err() {
                    has_header = true;
                    continue;
                }
            }

            // Take last column as signal value (supports "time,signal" generated files + headers)
            let parts: Vec<&str> = trimmed
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter(|s| !s.is_empty())
                .collect();
            if let Some(last) = parts.last() {
                if let Ok(v) = last.parse::<f64>() {
                    series.push(v);
                }
            }
        }
        if series.is_empty() {
            anyhow::bail!("no data");
        }
        let (known_p, known_s) = crate::generate::load_peaks_sidecar(path);
        let kind = SignalKind::from_path(path);
        let fs = kind.default_fs();
        let n_known = known_p.len();
        let n_sec = known_s.len();
        let n = series.len();
        self.loaded_signal = Some(LoadedSignal::with_meta(
            series,
            path.display().to_string(),
            fs,
            kind,
            known_p,
            known_s,
        ));
        if let Some(sig) = self.loaded_signal.as_mut() {
            if n_known >= 2 {
                sig.tachogram_source = TachogramSource::KnownPrimary;
                sig.rebuild_tachogram();
            }
        }
        self.peak_params = PeakDetectParams::for_kind(kind);
        self.peak_param_selection = 0;
        self.pending_peak_params = false;
        self.explore_scroll = 0;
        self.explore_view = ExploreView::Waveform;
        self.current_tab = Tab::Explore;
        self.current_workflow = Workflow::BioSym;
        self.status = if n_known + n_sec > 0 {
            format!(
                "Loaded {} ({} samples, {}) — known {}/{} — Explore  [k detect  i tachogram  e export]",
                path.display(),
                n,
                kind.label(),
                n_known,
                n_sec
            )
        } else {
            format!(
                "Loaded {} ({} samples) — Explore  [k detect  i tachogram  e export]",
                path.display(),
                n
            )
        };
        self.ensure_status_for_current_tab();
        Ok(())
    }

    pub fn load_ibi(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        use std::{
            fs::File,
            io::{
                BufRead,
                BufReader,
            },
        };
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut series = Vec::new();
        for line in reader.lines() {
            let line = line?;
            for tok in line.split_whitespace() {
                if let Ok(v) = tok.parse::<f64>() {
                    series.push(v);
                }
            }
        }
        if series.is_empty() {
            anyhow::bail!("no data");
        }
        let n = series.len();
        self.loaded_signal = Some(LoadedSignal::new(series, path.display().to_string()));
        self.current_tab = Tab::Explore;
        self.current_workflow = Workflow::BioSym;
        self.status = format!("Loaded IBI {} samples — switched to Explore", n);
        self.ensure_status_for_current_tab();
        Ok(())
    }

    pub fn inspect_csv_columns(&self, path: &PathBuf) -> anyhow::Result<usize> {
        use std::{
            fs::File,
            io::{
                BufRead,
                BufReader,
            },
        };
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut first = String::new();
        reader.read_line(&mut first)?;
        Ok(first.trim().split(',').count())
    }

    pub fn try_load_multicolumn(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        let n = self.inspect_csv_columns(path)?;
        if n <= 1 {
            return self.load_csv(path);
        }
        // simple load first col
        self.load_csv(path)?;
        self.status = format!("Multi col ({}), loaded col 0. (full picker later)", n);
        Ok(())
    }

    pub fn enter_column_picker(
        &mut self,
        path: PathBuf,
        data: Vec<Vec<f64>>,
        num_columns: usize,
        headers: Option<Vec<String>>,
    ) -> anyhow::Result<()> {
        self.pending_load = Some(PendingColumnLoad {
            path,
            data,
            columns: num_columns,
            headers,
        });
        self.status = format!("File has {} cols. Press 1-{} ", num_columns, num_columns);
        Ok(())
    }

    pub fn reset_loaded(&mut self) {
        if let Some(s) = &mut self.loaded_signal {
            s.reset();
        }
    }

    /// Switch workflow and set a sensible entry tab + status.
    pub fn switch_workflow(&mut self, wf: Workflow) {
        self.current_workflow = wf;
        match wf {
            Workflow::Home => {
                self.current_tab = Tab::Home;
                self.status =
                    "Home — 1/Enter=BioSym  2=LoadSym  3=SpatialSym  • Ctrl+H here".to_string();
            }
            Workflow::BioSym => {
                self.current_tab = if self.loaded_signal.is_some() {
                    Tab::Explore
                } else {
                    Tab::Import
                };
                self.status =
                    "BioSym — Import / Explore / Dynamics (RQA + cRQA + multiscale entropy)"
                        .to_string();
            }
            Workflow::SpatialSym => {
                self.current_tab = Tab::Spatial;
                self.spatial_view = SpatialView::Visualize;
                self.status = "SpatialSym — g:regen  i:import/generate  arrows:nav  (sub-views inside Spatial tab)".to_string();
            }
            Workflow::LoadSym => {
                self.current_tab = Tab::LoadSym;
                self.loadsym_view = LoadSymView::List;
                self.loadsym_selection = 0;
                // Refresh catalog if empty (or always try when entering workflow)
                let _ = crate::processing::try_load_loadsym_catalog(self);
                let cat = if self.loadsym_from_catalog {
                    format!("catalog {} days", self.daily_loads.len())
                } else {
                    "no catalog (g=demo, or symload ingest)".to_string()
                };
                self.status = format!(
                    "LoadSym — 1 Workout  2 Calendar  3 Optimization  • {}  • Ctrl+H home",
                    cat
                );
            }
        }
        self.ensure_status_for_current_tab();
    }

    /// Generalized status setter (extend as workflows grow).
    pub fn ensure_status_for_current_tab(&mut self) {
        if self.current_workflow == Workflow::Home {
            if !self.status.starts_with("Home") {
                self.status =
                    "Home — Select analysis path (1=BioSym, 2=LoadSym, 3=SpatialSym)".to_string();
            }
            return;
        }
        if self.current_tab != Tab::Spatial && self.status.starts_with("Spatial") {
            self.status = match self.current_tab {
                Tab::Import => {
                    "Import — / filter, ↑↓ select, Enter load, c convert, Ctrl+G generate"
                        .to_string()
                }
                Tab::Explore => {
                    "Explore — Ctrl+L live  p process  k peaks  K params  i tachogram  e export"
                        .to_string()
                }
                Tab::Dynamics => "Dynamics (RQA/cRQA + MSE)".to_string(),
                _ => "Symview".to_string(),
            };
        } else if self.current_tab == Tab::Spatial && !self.status.starts_with("Spatial") {
            let maxf = self
                .spatial_batch
                .as_ref()
                .map(|b| b.num_times().saturating_sub(1))
                .unwrap_or(0);
            self.status = format!("Spatial: frame {}/{}", self.spatial_frame_idx, maxf);
        } else if self.current_tab == Tab::LoadSym {
            if self.loadsym_view == LoadSymView::List {
                self.status =
                    "LoadSym — ↑↓ 1/2/3 select view (Workout / Calendar / Optimization) • Esc back"
                        .to_string();
            }
        }
    }

    /// Basic spatial CSV load using spatialsym loader + re-apply decision pipeline similar to seed.
    /// For real data we synthesize a minimal batch + decisions for viz reuse.
    pub fn load_spatial_csv(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        use symworx_spatialsym::{
            build_agent_trajectories,
            PlayingDimensions,
            Point2,
        };
        let path_str = path.to_string_lossy().to_string();
        let (times, trajs) = symworx_spatialsym::load_trajectories_csv(&path_str)
            .map_err(|e| anyhow::anyhow!("spatial load: {}", e))?;

        if trajs.is_empty() {
            anyhow::bail!("no trajectories in spatial csv");
        }

        // Build minimal synthetic-like structures so existing viz + summaries work.
        let n_agents = trajs.len();
        let n_steps = times.len().min(trajs[0].len());

        // Trim trajs to common length
        let trimmed: Vec<Vec<Point2>> = trajs
            .into_iter()
            .map(|mut v| {
                v.truncate(n_steps);
                v
            })
            .collect();

        // Fake groups / att directions / goal for compatibility
        let groups: Vec<u32> = (0..n_agents as u32).collect();
        let att = vec![symworx_spatialsym::Vec2::new(1., 0.); n_agents];
        let dims = Some(PlayingDimensions::new(105.0, 68.0));
        let goal_pos = vec![Point2::new(52.5, 0.0); n_agents];

        let ev_t = times.into_iter().take(n_steps).collect();
        let mut ev_f: Vec<Point2> = Vec::new();
        for t in 0..n_steps {
            let fx = trimmed
                .first()
                .and_then(|v| v.get(t))
                .map(|p| p.x)
                .unwrap_or(0.0);
            ev_f.push(Point2::new(fx + 2.0, 1.0));
        }
        let (batch, focal) =
            build_agent_trajectories(ev_t, trimmed, groups, att, ev_f, dims, Some(goal_pos));
        self.spatial_batch = Some(batch);
        self.spatial_focal = Some(focal);
        self.spatial_frame_idx = 0;

        // Rebuild decisions + labels for viz features
        if let (Some(b), Some(foc)) = (&self.spatial_batch, &self.spatial_focal) {
            let n_t = b.num_times();
            self.spatial_labels = Some(symworx_spatialsym::synthetic::generate_ground_truth(
                n_agents,
                n_t,
                "pass_then_press",
            ));
            let decs = b.classify_with_focal_and_params(foc, 0.5, 10.0, 0.8);
            self.spatial_decisions = Some(decs);
        }
        self.spatial_events = vec![(0, "start".to_string()), (n_steps / 2, "mid".to_string())];
        self.current_tab = Tab::Spatial;
        self.spatial_view = SpatialView::Visualize;
        self.current_workflow = Workflow::SpatialSym;
        self.status = format!(
            "Spatial loaded: {} ({} agents, {} steps)",
            path.display(),
            n_agents,
            n_steps
        );
        self.ensure_status_for_current_tab();
        Ok(())
    }

    pub fn refresh_spatial_list(&mut self) {
        // For now reuse main list + filter awareness; dedicated spatial filter separate.
        // Future: dedicated discovery.
    }
}
