// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Application state (`App`) — struct, construction, live stream, file list.

use std::path::PathBuf;

use ratatui::widgets::ListState;
use symworx_spatialsym::{
    decision::{
        AgentDecision,
        SpaceAction,
    },
    AgentTrajectories,
    Point2,
};

use super::{
    ActivityMetricsUiRow,
    CatalogRideRow,
    ExploreView,
    LoadSymView,
    LoadedSignal,
    MetricsChartMode,
    MetricsField,
    PeakDetectParams,
    PendingColumnLoad,
    PipelineModel,
    ResidualPanelMode,
    RqaParams,
    SignalKind,
    SpatialView,
    StatsLabResult,
    StatsLabTask,
    StatsView,
    Tab,
    WeeklyLoadRow,
    Workflow,
    WorkoutStream,
};

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
    /// Import: file delete confirmation (`x` arm · `y` confirm · `n`/Esc cancel).
    pub pending_delete: Option<PathBuf>,
    /// Legacy overlay flag (prefer `Tab::Generate`); cleared on cancel/nav.
    pub pending_generate: bool,
    /// BioSym Generate tab: which demo preset is highlighted (0..2).
    pub bio_gen_preset: usize,
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
    // StatsSym
    pub stats_view: StatsView,
    pub stats_selection: usize,
    /// Loaded numeric table for StatsSym (CSV via symworx-io).
    pub stats_table: Option<symworx_io::TableData>,
    /// Focused column index in Import/Lab preview.
    pub stats_col_focus: usize,
    /// Generate menu: which teaching preset is highlighted.
    pub stats_gen_preset: usize,
    /// Generate: sample size.
    pub stats_gen_n: usize,
    /// Generate: RNG seed.
    pub stats_gen_seed: u64,
    /// Generate: noise / softness scale.
    pub stats_gen_noise: f64,
    /// Last generate notes (ground truth) for status / Lab.
    pub stats_gen_notes: String,
    /// Lab: selected task index into [`StatsLabTask::ALL`].
    pub stats_lab_task: usize,
    /// Lab: feature / X column index.
    pub stats_lab_x_col: usize,
    /// Lab: target / Y column index.
    pub stats_lab_y_col: usize,
    /// Lab: residual panel Bland–Altman vs histogram.
    pub stats_residual_mode: ResidualPanelMode,
    /// Lab: last analysis result (charts + text).
    pub stats_lab_result: Option<StatsLabResult>,
    /// Pipeline: requested k-fold count on the train set (clamped at run time).
    pub stats_pipeline_k: usize,
    /// Poly: max degree to search (`0..=max`, subject to sample size).
    pub stats_poly_max_degree: usize,
    /// Pipeline: OLS vs Logistic evaluation model (`m` to cycle).
    pub stats_pipeline_model: PipelineModel,
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
            status: "Home — 1=BioSym  2=StatsSym  3=LoadSym  4=Spatial  • Ctrl+H home".to_string(),
            loaded_signal: None,
            pending_load: None,
            file_filter: String::new(),
            filter_mode: false,
            pending_delete: None,
            pending_generate: false,
            bio_gen_preset: 0,
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
            stats_view: StatsView::Import,
            stats_selection: 0,
            stats_table: None,
            stats_col_focus: 0,
            stats_gen_preset: 0,
            stats_gen_n: 200,
            stats_gen_seed: 42,
            stats_gen_noise: 0.5,
            stats_gen_notes: String::new(),
            stats_lab_task: 2, // default Regress for charting demos
            stats_lab_x_col: 0,
            stats_lab_y_col: 1,
            stats_residual_mode: ResidualPanelMode::BlandAltman,
            stats_lab_result: None,
            stats_pipeline_k: 5,
            stats_poly_max_degree: 3,
            stats_pipeline_model: PipelineModel::Ols,
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
    pub fn clear_esc_quit(&mut self) {
        self.esc_quit_pending = false;
    }
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
        self.pending_delete = None;
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
    pub fn poll_live(&mut self) {
        if let Some(live) = self.live.as_mut() {
            live.poll();
        }
    }
    pub fn is_live(&self) -> bool {
        self.live.is_some()
    }
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
    pub fn stop_live(&mut self) {
        if let Some(session) = self.live.take() {
            session.stop();
        }
    }
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
}
