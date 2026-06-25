use ratatui::widgets::ListState;
use symworx_spatialsym::{decision::{AgentDecision, SpaceAction}, synthetic, AgentTrajectories, Point2, Vec2, PlayingDimensions};
use std::path::PathBuf;

/// Top-level tabs for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Import,
    Explore,
    Dynamics,
    Spatial,
}

impl Tab {
    pub fn title(self) -> &'static str {
        match self {
            Tab::Import => "Import",
            Tab::Explore => "Explore",
            Tab::Dynamics => "Dynamics",
            Tab::Spatial => "Spatial",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Tab::Import => 0,
            Tab::Explore => 1,
            Tab::Dynamics => 2,
            Tab::Spatial => 3,
        }
    }
}

pub fn tab_titles() -> Vec<ratatui::text::Span<'static>> {
    vec!["1: Import", "2: Explore", "3: Dynamics", "4: Spatial"]
        .into_iter()
        .map(ratatui::text::Span::from)
        .collect()
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
    BasicStats { mean, std, min, max, median }
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
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            current_tab: Tab::Import,
            file_list: Vec::new(),
            list_state: ListState::default(),
            manual_path: String::new(),
            status: "Import — / filter, Ctrl+1/2/3/4 or Ctrl+←/→ tabs, Ctrl+G generate, q quit".to_string(),
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
        };
        app.refresh_file_list();
        if !app.file_list.is_empty() {
            app.list_state.select(Some(0));
        }
        app.seed_spatial_demo();
        app
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
                            if matches!(ext.to_lowercase().as_str(), "csv" | "txt" | "dat" | "bin" | "biosym") {
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
        let (batch, focal) =
            symworx_spatialsym::build_agent_trajectories(ev_t.clone(), ev_p, groups, att, ev_f.clone(), dims, Some(goal_pos));
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
        if vis.is_empty() { return; }
        let mut i = self.list_state.selected().unwrap_or(0);
        if i >= vis.len() { i = 0; }
        i = (i + 1) % vis.len();
        self.list_state.select(Some(i));
    }

    pub fn select_prev(&mut self) {
        let vis = self.visible_indices();
        if vis.is_empty() { return; }
        let mut i = self.list_state.selected().unwrap_or(0);
        if i >= vis.len() { i = 0; }
        i = if i == 0 { vis.len() - 1 } else { i - 1 };
        self.list_state.select(Some(i));
    }

    pub fn selected_path(&self) -> Option<&PathBuf> {
        let vis = self.visible_indices();
        let pos = self.list_state.selected()?;
        let orig = *vis.get(pos)?;
        self.file_list.get(orig)
    }

    pub fn ensure_status_for_current_tab(&mut self) {
        if self.current_tab != Tab::Spatial && self.status.starts_with("Spatial") {
            self.status = match self.current_tab {
                Tab::Import => "Import — / filter, ↑↓ select, Enter load, c convert, Ctrl+G generate".to_string(),
                Tab::Explore => "Explore — stats + sparkline (p to process)".to_string(),
                Tab::Dynamics => "Dynamics".to_string(),
                Tab::Spatial => "Spatial".to_string(),
            };
        } else if self.current_tab == Tab::Spatial && !self.status.starts_with("Spatial") {
            let maxf = self.spatial_batch.as_ref().map(|b| b.num_times().saturating_sub(1)).unwrap_or(0);
            self.status = format!("Spatial: frame {}/{}", self.spatial_frame_idx, maxf);
        }
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
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if ext == "csv" || ext == "txt" || ext == "dat" {
            return self.load_csv(path);
        }
        if ext == "ibi" {
            return self.load_ibi(path);
        }
        if let Ok(signal) = self.try_load_parquet(path) {
            self.loaded_signal = Some(signal);
            self.current_tab = Tab::Explore;
            self.status = format!("Loaded {} (switched to Explore)", path.display());
            self.ensure_status_for_current_tab();
            return Ok(());
        }
        anyhow::bail!("unsupported or failed: {}", path.display())
    }

    pub fn try_load_parquet(&self, path: &PathBuf) -> anyhow::Result<LoadedSignal> {
        // stub, use simple or assume
        Err(anyhow::anyhow!("parquet not fully wired here"))
    }

    pub fn load_csv(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut series = Vec::new();
        let mut has_header = false;
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() { continue; }
            if !has_header {
                if trimmed.parse::<f64>().is_err() { has_header = true; continue; }
            }
            if let Ok(v) = trimmed.parse::<f64>() { series.push(v); }
        }
        if series.is_empty() { anyhow::bail!("no data"); }
        self.loaded_signal = Some(LoadedSignal::new(series, path.display().to_string()));
        self.current_tab = Tab::Explore;
        self.status = format!("Loaded {} ({} samples) — switched to Explore", path.display(), self.loaded_signal.as_ref().map(|s|s.n_samples).unwrap_or(0));
        self.ensure_status_for_current_tab();
        Ok(())
    }

    pub fn load_ibi(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut series = Vec::new();
        for line in reader.lines() {
            let line = line?;
            for tok in line.split_whitespace() {
                if let Ok(v) = tok.parse::<f64>() { series.push(v); }
            }
        }
        if series.is_empty() { anyhow::bail!("no data"); }
        let n = series.len();
        self.loaded_signal = Some(LoadedSignal::new(series, path.display().to_string()));
        self.current_tab = Tab::Explore;
        self.status = format!("Loaded IBI {} samples — switched to Explore", n);
        self.ensure_status_for_current_tab();
        Ok(())
    }

    pub fn inspect_csv_columns(&self, path: &PathBuf) -> anyhow::Result<usize> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut first = String::new();
        reader.read_line(&mut first)?;
        Ok(first.trim().split(',').count())
    }

    pub fn try_load_multicolumn(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        let n = self.inspect_csv_columns(path)?;
        if n <= 1 { return self.load_csv(path); }
        // simple load first col
        self.load_csv(path)?;
        self.status = format!("Multi col ({}), loaded col 0. (full picker later)", n);
        Ok(())
    }

    pub fn enter_column_picker(&mut self, path: PathBuf, data: Vec<Vec<f64>>, num_columns: usize, headers: Option<Vec<String>>) -> anyhow::Result<()> {
        self.pending_load = Some(PendingColumnLoad { path, data, columns: num_columns, headers });
        self.status = format!("File has {} cols. Press 1-{} ", num_columns, num_columns);
        Ok(())
    }

    pub fn reset_loaded(&mut self) {
        if let Some(s) = &mut self.loaded_signal { s.reset(); }
    }
}
