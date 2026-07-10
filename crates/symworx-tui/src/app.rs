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

#[derive(Clone, Debug)]
pub struct LoadedSignal {
    pub original: Vec<f64>,
    pub current: Vec<f64>,
    pub name: String,
    pub n_samples: usize,
}

impl LoadedSignal {
    pub fn new(series: Vec<f64>, name: String) -> Self {
        let n = series.len();
        Self {
            original: series.clone(),
            current: series,
            name,
            n_samples: n,
        }
    }

    pub fn reset(&mut self) {
        self.current = self.original.clone();
        self.n_samples = self.current.len();
    }

    pub fn apply_processed(&mut self, new_series: Vec<f64>) {
        self.current = new_series;
        self.n_samples = self.current.len();
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
    pub help_mode: bool,
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
    pub loadsym_scroll: usize, // for long sessions / calendar scrolling
    /// Path of catalog last loaded into calendar (status only; may be None).
    pub loadsym_catalog_path: Option<PathBuf>,
    /// True when daily_loads came from SQLite catalog (not synthetic `g`).
    pub loadsym_from_catalog: bool,

    // Loaded activity (from .fit or activity CSV) — used by LoadSym Workout
    pub loaded_activity: Option<symworx_io::ActivityData>,
    pub activity_scroll: usize,
    pub activity_series: usize, // 0=power, 1=hr, 2=speed (fallback if not present)
    // Scrolling for BioSym Explore tab (long signals)
    pub explore_scroll: usize,
    // User exploration for LoadSym Workout: custom threshold + min duration (samples)
    pub workout_user_thresh: f64,
    pub workout_user_min_dur: usize,

    // LoadSym cycling power: FTP for TSS/NP/IF calculations (W)
    pub ftp: f64,
    // Directories to scan for .fit / activity files (in addition to ./data)
    pub loadsym_archive_dirs: Vec<PathBuf>,
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
            spatial_batch: None,
            spatial_focal: None,
            spatial_frame_idx: 0,
            spatial_labels: None,
            spatial_decisions: None,
            spatial_events: vec![],
            help_mode: false,
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
            loadsym_catalog_path: None,
            loadsym_from_catalog: false,
            loaded_activity: None,
            activity_scroll: 0,
            activity_series: 0,
            explore_scroll: 0,
            workout_user_thresh: 0.0,
            workout_user_min_dur: 3,
            ftp: 300.0,
            // Prefer personal velofit archive, then project-relative folders.
            loadsym_archive_dirs: symworx_io::default_activity_search_dirs(),
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

    pub fn clear_submodes(&mut self) {
        self.help_mode = false;
        self.pending_generate = false;
        self.filter_mode = false;
        self.pending_process = false;
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
        self.explore_scroll = 0;
        self.workout_user_thresh = 0.0;
        self.workout_user_min_dur = 3;
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
            synthetic::generate_event_driven(init, Point2::new(0.25, 0.), evs.clone(), 1.4, 0.1);
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
        self.loaded_signal = Some(LoadedSignal::new(series, path.display().to_string()));
        self.current_tab = Tab::Explore;
        self.current_workflow = Workflow::BioSym;
        self.status = format!(
            "Loaded {} ({} samples) — switched to Explore",
            path.display(),
            self.loaded_signal
                .as_ref()
                .map(|s| s.n_samples)
                .unwrap_or(0)
        );
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
                    format!(
                        "catalog {} days",
                        self.daily_loads.len()
                    )
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
                Tab::Explore => "Explore — stats + sparkline (p to process)".to_string(),
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
