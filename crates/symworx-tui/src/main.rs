// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! symworx-tui — Terminal interface for exploring biosym/physiological signals
//! and nonlinear dynamics (RQA, embedding, etc.).
//!
//! Features tabs for Import (with demo data generation from symworx-biosym),
//! Explore (stats + processing + visualization), and Dynamics.

use anyhow::Result;
use crossterm::event::{
    self,
    KeyCode,
    KeyEventKind,
    KeyModifiers,
};
use ratatui::{
    layout::{
        Constraint,
        Layout,
        Rect,
    },
    style::{
        Color,
        Modifier,
        Style,
        Stylize,
    },
    text::Span,
    widgets::{
        Block,
        Borders,
        Cell,
        List,
        ListItem,
        ListState,
        Paragraph,
        Row,
        Sparkline,
        Table,
        Tabs,
    },
    DefaultTerminal,
    Frame,
};
mod convert;
mod generate;

use std::{
    fs,
    path::PathBuf,
    time::Duration,
};

use symworx_spatialsym::{
    decision::SpaceAction,
    synthetic,
    AgentTrajectories,
    Point2,
    Vec2,
};

/// Top-level tabs for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    /// Import / file discovery and conversion
    Import,
    /// Basic statistics, processing, filtering, simple visualization, edim/fnn
    Explore,
    /// Nonlinear dynamics (RQA, recurrence plots, etc.)
    Dynamics,
    /// Spatial trajectory analysis (synthetic, decisions, frames)
    Spatial,
}

impl Tab {
    fn title(self) -> &'static str {
        match self {
            Tab::Import => "Import",
            Tab::Explore => "Explore",
            Tab::Dynamics => "Dynamics",
            Tab::Spatial => "Spatial",
        }
    }

    fn index(self) -> usize {
        match self {
            Tab::Import => 0,
            Tab::Explore => 1,
            Tab::Dynamics => 2,
            Tab::Spatial => 3,
        }
    }
}

/// Main application state
struct App {
    /// Currently selected top-level tab
    current_tab: Tab,
    /// List of discoverable signal files in the data directory (or current dir)
    file_list: Vec<PathBuf>,
    /// Which file is highlighted in the list
    list_state: ListState,
    /// Manual path input (user can type a full path)
    manual_path: String,
    /// Status / error messages
    status: String,
    /// Currently loaded signal (available across Explore and Dynamics tabs)
    loaded_signal: Option<LoadedSignal>,

    // --- Import tab specific state ---
    /// When user tries to load a multi-column file, we store the data here temporarily

    // --- Spatial tab (from symworx-spatialsym) ---
    /// Demo synthetic batch for spatial analysis
    spatial_batch: Option<symworx_spatialsym::AgentTrajectories>,
    spatial_focal: Option<Vec<symworx_spatialsym::Point2>>,
    spatial_frame_idx: usize,
    spatial_labels: Option<Vec<Vec<symworx_spatialsym::decision::SpaceAction>>>,
    /// and ask them to pick a column.
    pending_load: Option<PendingColumnLoad>,

    /// Live filter string for the file list in the Import tab
    file_filter: String,

    /// When true, the user is actively typing in the file filter (entered via `/`)
    filter_mode: bool,

    /// When true, the user is in "generate demo data" mode (very lightweight submenu)
    pending_generate: bool,

    // --- Explore tab processing controls ---
    /// When true, the processing menu is active in the Explore tab
    pending_process: bool,
    /// 0 = Moving Average, 1 = Median Filter, 2 = Detrend
    process_selection: usize,
    /// Window size for moving average / median filter
    process_window: usize,
}

/// Temporary state while user is choosing which column to load from a file.
#[derive(Clone)]
struct PendingColumnLoad {
    path: PathBuf,
    data: Vec<Vec<f64>>,
    columns: usize,
    /// Optional header names (one per column). If present, use these in the picker instead of "Column N".
    headers: Option<Vec<String>>,
}

// Screen enum removed — we now use Tab + loaded_signal for navigation and state.

fn format_file_size(bytes: u64) -> String {
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

/// Represents a loaded time series with support for processing.
#[derive(Clone, Debug)]
struct LoadedSignal {
    /// Original series as loaded from file
    pub original: Vec<f64>,
    /// Current (possibly filtered/interpolated) series being analyzed
    pub current: Vec<f64>,
    /// Basic metadata
    pub name: String,
    /// Number of samples in current view
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

    /// Reset to original data
    pub fn reset(&mut self) {
        self.current = self.original.clone();
        self.n_samples = self.current.len();
    }

    /// Replace the current series (after filtering, interpolation, etc.)
    pub fn apply_processed(&mut self, new_series: Vec<f64>) {
        self.current = new_series;
        self.n_samples = self.current.len();
    }
}

/// Simple summary statistics
#[derive(Debug, Clone, Default)]
struct BasicStats {
    mean: f64,
    std: f64,
    min: f64,
    max: f64,
    median: f64,
}

fn compute_basic_stats(data: &[f64]) -> BasicStats {
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

impl App {
    fn new() -> Self {
        let mut app = Self {
            current_tab: Tab::Import,
            file_list: Vec::new(),
            list_state: ListState::default(),
            manual_path: String::new(),
            status: "Import tab — / to filter, Ctrl+1/2/3/4 or Ctrl+←/→ to switch tabs."
                .to_string(),
            loaded_signal: None,
            pending_load: None,
            file_filter: String::new(),
            filter_mode: false,
            pending_generate: false,
            pending_process: false,
            process_selection: 0,
            process_window: 5,
            // Spatial demo init
            spatial_batch: None,
            spatial_focal: None,
            spatial_frame_idx: 0,
            spatial_labels: None,
        };
        app.refresh_file_list();
        if !app.file_list.is_empty() {
            app.list_state.select(Some(0));
        }
        // Seed a synthetic spatial demo (options 1-3)
        app.seed_spatial_demo();
        app
    }

    fn seed_spatial_demo(&mut self) {
        // Use event-driven synthetic (option 3) with metadata
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
            synthetic::generate_event_driven(init, Point2::new(0.25, 0.), evs, 1.4, 0.1);
        let groups = vec![0u32, 0, 1];
        let att = vec![Vec2::new(1., 0.), Vec2::new(1., 0.), Vec2::new(-1., 0.)];
        let (batch, focal) =
            symworx_spatialsym::build_agent_trajectories(ev_t, ev_p, groups, att, ev_f.clone());
        let n_steps = batch.num_times();
        self.spatial_batch = Some(batch);
        self.spatial_focal = Some(focal);
        self.spatial_frame_idx = 0;
        // Ground truth for this scenario (simple)
        self.spatial_labels = Some(symworx_spatialsym::synthetic::generate_ground_truth(
            3,
            n_steps,
            "pass_then_press",
        ));
    }

    /// Scan for likely biosignal files.
    /// TODO: Replace/extend this with discovery logic from your Symworx IO or biosym format.
    fn refresh_file_list(&mut self) {
        self.file_list.clear();
        self.file_filter.clear(); // Clear filter on refresh for fresh view

        // Look in ./data first, then current directory
        let candidates = ["./data", "."];

        for dir in candidates {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            // Common biosignal / physiological file extensions
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

        // Sort for stable ordering
        self.file_list.sort();
    }

    fn selected_file(&self) -> Option<&PathBuf> {
        self.list_state
            .selected()
            .and_then(|i| self.file_list.get(i))
    }

    fn load_selected_or_manual(&mut self) -> Result<()> {
        let path = if !self.manual_path.trim().is_empty() {
            PathBuf::from(self.manual_path.trim())
        } else if let Some(p) = self.selected_file() {
            p.clone()
        } else {
            self.status = "No file selected and no manual path entered".to_string();
            return Ok(());
        };

        if !path.exists() {
            self.status = format!("File not found: {}", path.display());
            return Ok(());
        }

        // Real loading via symworx-io (re-exported through symworx-core)
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext.eq_ignore_ascii_case("ibi") || ext.eq_ignore_ascii_case("biosym") {
                // IBI files are single-series by nature
                use symworx_core::io::read_ibi;
                let records = read_ibi(path.to_str().unwrap())?;
                let series: Vec<f64> = records.into_iter().map(|r| r.rr_ms as f64).collect();

                if series.is_empty() {
                    self.status = "IBI file contained no records".to_string();
                    return Ok(());
                }

                let signal = LoadedSignal::new(series.clone(), "RR intervals".to_string());
                self.loaded_signal = Some(signal);
                self.current_tab = Tab::Explore;
                self.status = format!(
                    "Loaded IBI (RR intervals): {} samples — switched to Explore tab",
                    series.len()
                );
                self.manual_path.clear();
                return Ok(());
            }
        }

        // General tabular file (CSV, Parquet, etc.)
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());

        if ext.as_deref() == Some("csv") {
            // Smarter CSV handling: support headers for better UX in column picker
            use csv::ReaderBuilder;

            let mut rdr = ReaderBuilder::new().has_headers(true).from_path(&path)?;

            let headers = rdr
                .headers()?
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();

            let mut data: Vec<Vec<f64>> = Vec::new();

            for result in rdr.records() {
                let record = result?;
                let row: Vec<f64> = record
                    .iter()
                    .filter_map(|v| v.parse::<f64>().ok())
                    .collect();

                if !row.is_empty() {
                    data.push(row);
                }
            }

            if data.is_empty() {
                self.status = "File contained no numeric data after header".to_string();
                return Ok(());
            }

            let num_columns = data[0].len();

            if num_columns == 0 {
                self.status = "File contained no numeric columns".to_string();
                return Ok(());
            }

            if num_columns == 1 {
                let series: Vec<f64> = data
                    .into_iter()
                    .filter_map(|row| row.first().copied())
                    .collect();
                let name = headers
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "series".to_string());

                let signal = LoadedSignal::new(series.clone(), name);
                self.loaded_signal = Some(signal);
                self.current_tab = Tab::Explore;
                self.status = format!(
                    "Loaded {} ({} samples, single column) — switched to Explore tab",
                    path.display(),
                    series.len()
                );
                self.manual_path.clear();
                return Ok(());
            }

            // Multiple columns — enter column selection with nice header names
            let col_desc = if headers.len() == num_columns {
                format!(" (headers: {})", headers.join(", "))
            } else {
                String::new()
            };

            self.pending_load = Some(PendingColumnLoad {
                path: path.clone(),
                data,
                columns: num_columns,
                headers: if headers.len() == num_columns {
                    Some(headers)
                } else {
                    None
                },
            });

            self.status = format!(
                "File has {} columns{}. Press 1-{} to choose which column (or Esc to cancel).",
                num_columns, col_desc, num_columns
            );
            return Ok(());
        }

        // Fallback for Parquet / other formats (no header support yet)
        use symworx_core::io::load_any;
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Path contains invalid UTF-8: {}", path.display()))?;
        let data = load_any(path_str)?;

        if data.is_empty() {
            self.status = "File contained no data".to_string();
            return Ok(());
        }

        let num_columns = data[0].len();

        if num_columns == 0 {
            self.status = "File contained no numeric columns".to_string();
            return Ok(());
        }

        if num_columns == 1 {
            let series: Vec<f64> = data
                .into_iter()
                .filter_map(|row| row.first().copied())
                .collect();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("series")
                .to_string();

            let signal = LoadedSignal::new(series.clone(), name);
            self.loaded_signal = Some(signal);
            self.current_tab = Tab::Explore;
            self.status = format!(
                "Loaded {} ({} samples, single column) — switched to Explore tab",
                path.display(),
                series.len()
            );
            self.manual_path.clear();
            return Ok(());
        }

        // Multiple columns — enter column selection (no header names available for Parquet/other)
        self.pending_load = Some(PendingColumnLoad {
            path: path.clone(),
            data,
            columns: num_columns,
            headers: None,
        });

        self.status = format!(
            "File has {} columns. Press 1-{} to choose which column (or Esc to cancel).",
            num_columns, num_columns
        );
        Ok(())
    }
}

fn main() -> Result<()> {
    color_eyre::install().expect("Failed to install color_eyre");
    ratatui::run(app)?;
    Ok(())
}

fn app(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    let mut app = App::new();
    let tick_rate = Duration::from_millis(100);

    loop {
        terminal.draw(|frame| ui(frame, &app))?;

        if event::poll(tick_rate)? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && handle_key(&mut app, key.code, key.modifiers)
                {
                    return Ok(()); // quit
                }
            }
        }
    }
}

/// Returns true if we should quit
fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    // Dedicated quit key (always works, even in sub-modes)
    if code == KeyCode::Char('q') {
        return true;
    }

    // Global keys first (Esc is intentionally NOT a global quit here
    // so it can be used to cancel sub-modes like column selection)
    match code {
        // Tab switching via Ctrl+1 / Ctrl+2 / Ctrl+3
        KeyCode::Char('1') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_tab = Tab::Import
        }
        KeyCode::Char('2') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_tab = Tab::Explore
        }
        KeyCode::Char('3') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_tab = Tab::Dynamics
        }
        KeyCode::Char('4') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_tab = Tab::Spatial
        }

        // Tab switching via Ctrl+Left / Ctrl+Right (keep this behavior)
        KeyCode::Left if modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_tab = match app.current_tab {
                Tab::Import => Tab::Import,
                Tab::Explore => Tab::Import,
                Tab::Dynamics => Tab::Explore,
                Tab::Spatial => Tab::Dynamics,
            };
        }
        KeyCode::Right if modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_tab = match app.current_tab {
                Tab::Import => Tab::Explore,
                Tab::Explore => Tab::Dynamics,
                Tab::Dynamics => Tab::Spatial,
                Tab::Spatial => Tab::Spatial,
            };
        }

        // Generate demo data with Ctrl+G
        KeyCode::Char('g') | KeyCode::Char('G') if modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_tab != Tab::Import {
                app.current_tab = Tab::Import;
            }
            app.pending_generate = true;
            app.manual_path.clear();
            app.file_filter.clear();
            app.status = "Generate demo data: 1 = Resting PPG   2 = Respiration   3 = Stride intervals   Esc = cancel".to_string();
            return false;
        }

        _ => {}
    }

    // Tab-specific handling
    match app.current_tab {
        Tab::Import => handle_import_keys(app, code, modifiers),
        Tab::Explore => handle_explore_keys(app, code, modifiers),
        Tab::Dynamics => handle_dynamics_keys(app, code),
        Tab::Spatial => handle_spatial_keys(app, code, modifiers),
    }
}

fn handle_spatial_keys(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    if let Some(batch) = &app.spatial_batch {
        let max_frame = batch.num_times().saturating_sub(1);
        match code {
            KeyCode::Left => {
                if app.spatial_frame_idx > 0 {
                    app.spatial_frame_idx -= 1;
                }
                app.status = format!("Spatial frame {} / {}", app.spatial_frame_idx, max_frame);
            }
            KeyCode::Right => {
                if app.spatial_frame_idx < max_frame {
                    app.spatial_frame_idx += 1;
                }
                app.status = format!("Spatial frame {} / {}", app.spatial_frame_idx, max_frame);
            }
            KeyCode::Char('g') | KeyCode::Char('G')
                if modifiers.contains(KeyModifiers::CONTROL) =>
            {
                // regenerate demo
                app.seed_spatial_demo();
                app.status = "Regenerated spatial synthetic demo".to_string();
            }
            _ => {}
        }
    }
    true
}

fn handle_import_keys(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    // === Highest priority: modal sub-screens completely own input while active ===

    // Demo generation submenu (entered via Ctrl+G). Numbers must NOT leak into manual_path.
    if app.pending_generate {
        match code {
            KeyCode::Char('1') => {
                if let Err(e) = generate_demo_and_load(app, generate::DemoPreset::RestingPPG) {
                    app.status = format!("Generation failed: {e}");
                }
                app.pending_generate = false;
                app.manual_path.clear();
                return false;
            }
            KeyCode::Char('2') => {
                if let Err(e) = generate_demo_and_load(app, generate::DemoPreset::LightRespiration)
                {
                    app.status = format!("Generation failed: {e}");
                }
                app.pending_generate = false;
                app.manual_path.clear();
                return false;
            }
            KeyCode::Char('3') => {
                if let Err(e) = generate_demo_and_load(app, generate::DemoPreset::SimpleStride) {
                    app.status = format!("Generation failed: {e}");
                }
                app.pending_generate = false;
                app.manual_path.clear();
                return false;
            }
            KeyCode::Esc => {
                app.pending_generate = false;
                app.manual_path.clear();
                app.status = "Demo generation cancelled".to_string();
                return false;
            }
            _ => {}
        }
        // Swallow every other key while the generate overlay is visible.
        // This prevents digits (and everything else) from being appended to manual_path.
        return false;
    }

    // Column selection for multi-column files (also owns digits + Esc)
    if let Some(pending) = &app.pending_load {
        if let KeyCode::Char(c) = code {
            if let Some(digit) = c.to_digit(10) {
                let col_index = (digit as usize).saturating_sub(1); // 1-based input

                if col_index < pending.columns {
                    let series: Vec<f64> = pending
                        .data
                        .iter()
                        .filter_map(|row| row.get(col_index).copied())
                        .collect();

                    let col_name = if let Some(hs) = &pending.headers {
                        if col_index < hs.len() {
                            hs[col_index].clone()
                        } else {
                            format!("Column {}", col_index + 1)
                        }
                    } else {
                        format!("Column {}", col_index + 1)
                    };
                    let signal = LoadedSignal::new(series.clone(), col_name.clone());

                    app.loaded_signal = Some(signal);
                    app.current_tab = Tab::Explore;
                    app.status = format!(
                        "Loaded {} ({} samples) — switched to Explore tab",
                        col_name,
                        series.len()
                    );
                    app.pending_load = None;
                    app.manual_path.clear();
                    return false;
                } else {
                    app.status = format!("Invalid column. Choose 1-{}", pending.columns);
                }
            }
        }

        if code == KeyCode::Esc {
            app.pending_load = None;
            app.status = "Column selection cancelled".to_string();
            return false;
        }
        // For other keys while picker is up we fall through (harmless; overlay is shown).
    }

    // Early reliable refresh (works even while typing in manual_path or filter).
    // F5 and Ctrl+R are the documented ways; we prioritize them hard.
    if code == KeyCode::F(5)
        || (matches!(code, KeyCode::Char('r') | KeyCode::Char('R'))
            && modifiers.contains(KeyModifiers::CONTROL))
    {
        app.refresh_file_list();
        app.status = "File list refreshed (Ctrl+R)".to_string();
        return false;
    }

    // Filter mode takes priority for normal typing (entered with /)
    if app.filter_mode {
        match code {
            KeyCode::Char(c) if c.is_ascii() => {
                app.file_filter.push(c);
                if !app.file_list.is_empty() {
                    app.list_state.select(Some(0));
                }
            }
            KeyCode::Backspace => {
                app.file_filter.pop();
                if !app.file_list.is_empty() {
                    app.list_state.select(Some(0));
                }
            }
            KeyCode::Esc | KeyCode::Enter => {
                app.filter_mode = false;
                app.status = if app.file_filter.is_empty() {
                    "Filter cleared".to_string()
                } else {
                    format!("Filtering: \"{}\"", app.file_filter)
                };
            }
            _ => {}
        }
        return false;
    }

    match code {
        // List navigation
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(selected) = app.list_state.selected() {
                if selected > 0 {
                    app.list_state.select(Some(selected - 1));
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let len = app.file_list.len();
            if len > 0 {
                let next = app
                    .list_state
                    .selected()
                    .map_or(0, |s| (s + 1).min(len - 1));
                app.list_state.select(Some(next));
            }
        }

        KeyCode::Char('c') => {
            if let Some(p) = app.selected_file() {
                let output = p.with_extension("csv");
                match convert::convert_to_csv(p, Some(&output)) {
                    Ok(()) => {
                        app.status = format!("Converted → {}", output.display());
                        app.refresh_file_list();
                    }
                    Err(e) => app.status = format!("Conversion failed: {e}"),
                }
            } else {
                app.status = "No file selected for conversion (press 'c')".to_string();
            }
            return false;
        }

        // Explicit filter mode (press / to enter)
        KeyCode::Char('/') => {
            app.filter_mode = true;
            app.file_filter.clear();
            app.status = "Filter mode — type to search files, Esc/Enter to exit".to_string();
        }

        // Load action (switches to Explore tab automatically)
        KeyCode::Enter => {
            if let Err(e) = app.load_selected_or_manual() {
                app.status = format!("Error loading file: {e}");
            }
        }

        // Manual path input (now safe: digits for generate/column picker are handled above)
        KeyCode::Char(c) if c.is_ascii() => {
            if !app.file_filter.is_empty() {
                app.file_filter.clear();
            }
            app.manual_path.push(c);
        }
        KeyCode::Backspace if !app.manual_path.is_empty() => {
            app.manual_path.pop();
        }

        _ => {}
    }

    // Esc in normal mode (no active sub-mode) quits the TUI
    if code == KeyCode::Esc {
        return true;
    }

    false
}

/// Helper: generate using biosym preset, save to data/, load it, and switch to Explore tab.
fn generate_demo_and_load(app: &mut App, preset: generate::DemoPreset) -> anyhow::Result<()> {
    let data_dir = std::path::Path::new("data");
    let path = generate::generate_and_save(preset, data_dir)?;

    // Load the *signal* column (second column) using header-aware reader.
    // All demo files have headers and put the interesting data (ppg/flow/stride) in column 1.
    let series: Vec<f64> = {
        use csv::ReaderBuilder;
        let mut rdr = ReaderBuilder::new().has_headers(true).from_path(&path)?;
        let mut out = Vec::new();
        for record in rdr.records().flatten() {
            if let Some(val_stgr) = record.get(1) {
                if let Ok(v) = val_stgr.parse::<f64>() {
                    out.push(v);
                }
            }
        }
        out
    };

    if series.is_empty() {
        return Err(anyhow::anyhow!(
            "Generated file contained no usable signal data in column 1"
        ));
    }

    let signal = LoadedSignal::new(series.clone(), preset.name().to_string());
    app.loaded_signal = Some(signal);
    app.current_tab = Tab::Explore;

    // Refresh the file list in the Import tab so the new file appears when user returns
    app.refresh_file_list();

    app.status = format!(
        "Generated {} → loaded {} samples. Switched to Explore tab. (Press Ctrl+1 to return to Import)",
        preset.name(),
        series.len()
    );

    Ok(())
}

/// Simple moving average filter.
fn moving_average(data: &[f64], window: usize) -> Vec<f64> {
    if window <= 1 || data.len() < window {
        return data.to_vec();
    }
    let w = window as f64;
    let mut out = Vec::with_capacity(data.len());
    let mut sum = 0.0;

    for i in 0..data.len() {
        sum += data[i];
        if i >= window {
            sum -= data[i - window];
        }
        if i + 1 >= window {
            out.push(sum / w);
        } else {
            out.push(data[i]); // ramp up
        }
    }
    out
}

/// Median filter (odd window preferred).
fn median_filter(data: &[f64], window: usize) -> Vec<f64> {
    if window <= 1 || data.len() < window {
        return data.to_vec();
    }
    let half = window / 2;
    let mut out = Vec::with_capacity(data.len());

    for i in 0..data.len() {
        let start = i.saturating_sub(half);
        let end = (i + half + 1).min(data.len());
        let mut window_vals: Vec<f64> = data[start..end].to_vec();
        window_vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let med = window_vals[window_vals.len() / 2];
        out.push(med);
    }
    out
}

/// Simple detrending: subtract the mean.
fn detrend_mean(data: &[f64]) -> Vec<f64> {
    if data.is_empty() {
        return vec![];
    }
    let mean = data.iter().sum::<f64>() / data.len() as f64;
    data.iter().map(|v| v - mean).collect()
}

fn handle_explore_keys(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    // === Processing menu (must be checked early to prevent key swallowing) ===
    if app.pending_process {
        if let Some(signal) = &app.loaded_signal {
            match code {
                KeyCode::Char('1') => {
                    app.process_selection = 0;
                    app.status = format!(
                        "Moving Average — window: {} (←/→ or -/+ to adjust, Enter to apply)",
                        app.process_window
                    );
                }
                KeyCode::Char('2') => {
                    app.process_selection = 1;
                    app.status = format!(
                        "Median Filter — window: {} (←/→ or -/+ to adjust, Enter to apply)",
                        app.process_window
                    );
                }
                KeyCode::Char('3') => {
                    app.process_selection = 2;
                    app.status =
                        "Detrend (subtract mean) — press Enter to apply, Esc to cancel".to_string();
                }
                KeyCode::Left | KeyCode::Char('-') | KeyCode::Char('_') => {
                    if app.process_selection < 2 {
                        app.process_window = (app.process_window.saturating_sub(1)).max(3);
                        let name = if app.process_selection == 0 {
                            "Moving Average"
                        } else {
                            "Median Filter"
                        };
                        app.status =
                            format!("{} — window: {} (Enter to apply)", name, app.process_window);
                    }
                }
                KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('=') => {
                    if app.process_selection < 2 {
                        app.process_window = (app.process_window + 1).min(101);
                        let name = if app.process_selection == 0 {
                            "Moving Average"
                        } else {
                            "Median Filter"
                        };
                        app.status =
                            format!("{} — window: {} (Enter to apply)", name, app.process_window);
                    }
                }
                KeyCode::Enter => {
                    let new_series = match app.process_selection {
                        0 => moving_average(&signal.current, app.process_window),
                        1 => median_filter(&signal.current, app.process_window),
                        2 => detrend_mean(&signal.current),
                        _ => signal.current.clone(),
                    };

                    // Apply using the existing mechanism
                    if let Some(s) = &mut app.loaded_signal {
                        s.apply_processed(new_series);
                    }

                    app.pending_process = false;
                    app.status = "Processing applied. Press 'r' to reset to original.".to_string();
                    return false;
                }
                KeyCode::Esc => {
                    app.pending_process = false;
                    app.status = "Processing cancelled".to_string();
                    return false;
                }
                _ => {}
            }
        } else {
            app.pending_process = false;
        }
        return false; // Swallow all keys while processing menu is open
    }

    // Normal Explore keys (future expansion)
    match code {
        // Open processing menu
        KeyCode::Char('p') | KeyCode::Char('P') => {
            if app.loaded_signal.is_some() {
                app.pending_process = true;
                app.process_selection = 0;
                app.process_window = 5;
                app.status = "Processing: 1=Moving Avg  2=Median  3=Detrend   ←/→ adjust window   Enter=Apply   Esc=Cancel".to_string();
            } else {
                app.status = "Load a signal first (Import tab)".to_string();
            }
            return false;
        }
        // Reset to original
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if let Some(s) = &mut app.loaded_signal {
                s.reset();
                app.status = "Reset to original signal".to_string();
            }
            return false;
        }
        _ => {}
    }

    false
}

fn handle_dynamics_keys(_app: &mut App, _code: KeyCode) -> bool {
    // TODO: RQA parameter tuning, recurrence plot navigation, etc.
    false
}

fn ui(frame: &mut Frame, app: &App) {
    let main_layout = Layout::vertical([
        Constraint::Length(1), // tab bar
        Constraint::Length(1), // action bar / key hints
        Constraint::Min(8),    // main content
        Constraint::Length(2), // footer / status
    ])
    .split(frame.area());

    // Top tab bar
    let tab_titles: Vec<Span> = vec!["1: Import", "2: Explore", "3: Dynamics", "4: Spatial"]
        .into_iter()
        .map(Span::from)
        .collect();

    let tabs = Tabs::new(tab_titles)
        .block(Block::new().borders(Borders::BOTTOM))
        .select(app.current_tab.index())
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, main_layout[0]);

    // Action bar / key legend (contextual per tab + mode)
    let action_bar = render_action_bar(app);
    frame.render_widget(action_bar, main_layout[1]);

    // Dispatch main content based on current tab
    match app.current_tab {
        Tab::Import => render_import_tab(frame, app, main_layout[2]),
        Tab::Explore => render_explore_tab(frame, app, main_layout[2]),
        Tab::Dynamics => render_dynamics_tab(frame, app, main_layout[2]),
        Tab::Spatial => render_spatial_tab(frame, app, main_layout[2]),
    }

    // Footer / status (kept minimal now that we have a dedicated action bar)
    let footer = Paragraph::new(format!(" {}  •  q: Quit", app.status))
        .centered()
        .dim()
        .block(Block::new().borders(Borders::TOP));
    frame.render_widget(footer, main_layout[3]);
}

/// Renders a contextual action bar showing available keybindings.
/// This greatly improves discoverability compared to hidden single-key commands.
fn render_action_bar(app: &App) -> Paragraph<'_> {
    let (text, style) = match app.current_tab {
        Tab::Import => {
            if app.pending_generate {
                (
                    "  [1] PPG   [2] Respiration   [3] Stride intervals   [Esc] Cancel",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )
            } else if app.filter_mode {
                (
                    "  Filtering...  [Esc] or [Enter] to exit filter",
                    Style::default().fg(Color::Cyan),
                )
            } else {
                (
                    "  [/] Filter   [Ctrl+G] Generate demo   [c] Convert   [Enter] Load   [↑↓] Navigate",
                    Style::default().fg(Color::DarkGray),
                )
            }
        }
        Tab::Explore => (
            "  [p] Process (MA / Median / Detrend)   [r] Reset to original   Stats + Sparkline active",
            Style::default().fg(Color::DarkGray),
        ),
        Tab::Dynamics => (
            "  [Coming soon: RQA, Recurrence Plots, Nonlinear Analysis]",
            Style::default().fg(Color::DarkGray),
        ),
        Tab::Spatial => (
            "  [←→] Change frame   [g] Regenerate   [i] Infer carrier   [l] Labels",
            Style::default().fg(Color::DarkGray),
        ),
    };

    Paragraph::new(text)
        .style(style)
        .block(Block::new().borders(Borders::BOTTOM))
}

/// Render the Import tab (file discovery + conversion)
fn render_import_tab(frame: &mut Frame, app: &App, area: Rect) {
    // Column selection mode (smarter with headers when available)
    if let Some(pending) = &app.pending_load {
        let mut lines = vec![
            format!("\n\nFile: {}\n", pending.path.display()),
            format!("This file contains {} columns.\n\n", pending.columns),
            "Press the number key for the column you want to load as the main series:\n\n"
                .to_string(),
        ];

        if let Some(headers) = &pending.headers {
            for (i, name) in headers.iter().enumerate() {
                lines.push(format!("  {} = {} (column {})\n", i + 1, name, i));
            }
        } else {
            for i in 0..pending.columns {
                lines.push(format!("  {} = Column {}\n", i + 1, i));
            }
        }

        lines.push("\nPress Esc to cancel.".to_string());

        let content = Paragraph::new(lines.join("")).centered().block(
            Block::new()
                .title(" Select Column to Load ")
                .borders(Borders::ALL)
                .border_style(Color::Yellow),
        );

        frame.render_widget(content, area);
        return;
    }

    // Lightweight demo data generation mode (g key)
    if app.pending_generate {
        let content = Paragraph::new(
            "\n\nGenerate realistic demo data using symworx-biosym\n\n\
             1 = Resting PPG (30s)\n\
             2 = Light activity respiration\n\
             3 = Simple stride/gait intervals\n\n\
             Press a number (1-3) or Esc to cancel.",
        )
        .centered()
        .block(
            Block::new()
                .title(" Generate Demo Data ")
                .borders(Borders::ALL)
                .border_style(Color::Magenta),
        );
        frame.render_widget(content, area);
        return;
    }

    // Split: top instructions, middle file list + info, bottom filter hint
    let chunks = Layout::vertical([
        Constraint::Length(4), // instructions
        Constraint::Min(8),    // file list + info panel
        Constraint::Length(2), // filter hint
    ])
    .split(area);

    // Instructions
    let filter_hint = if app.file_filter.is_empty() {
        " / : Filter   Ctrl+G : Generate   c : Convert   Ctrl+R : Refresh   Enter : Load"
    } else {
        "Filtering active • Esc or Backspace to clear filter"
    };

    let input_block = Block::new()
        .title(format!(" Import & File Discovery — {} ", filter_hint))
        .borders(Borders::ALL)
        .border_style(Color::Yellow);

    let input_text = if app.manual_path.is_empty() {
        "→  Type a full path and press Enter, or select from the list below"
            .dim()
            .to_string()
    } else {
        format!("→  {}", app.manual_path)
    };

    let input_para = Paragraph::new(input_text).block(input_block);
    frame.render_widget(input_para, chunks[0]);

    // Main content area: file list on left, info on right
    let list_chunks = Layout::horizontal([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(chunks[1]);

    // === Filtered file list ===
    let filtered_files: Vec<&PathBuf> = if app.file_filter.is_empty() {
        app.file_list.iter().collect()
    } else {
        let filter_lower = app.file_filter.to_lowercase();
        app.file_list
            .iter()
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|name| name.to_lowercase().contains(&filter_lower))
                    .unwrap_or(false)
            })
            .collect()
    };

    let list_title = format!(
        " Files ({}/{}) — r: Refresh  •  c: Convert selected ",
        filtered_files.len(),
        app.file_list.len()
    );

    let list_block = Block::new().title(list_title).borders(Borders::ALL);

    let items: Vec<ListItem> = filtered_files
        .iter()
        .map(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("???");

            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_uppercase();

            let size_str = std::fs::metadata(p)
                .map(|m| format_file_size(m.len()))
                .unwrap_or_else(|_| "?".to_string());

            let display = format!("{}  [{}]  {}", name, ext, size_str);
            ListItem::new(display)
        })
        .collect();

    let _list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(_list, list_chunks[0], &mut app.list_state.clone());

    // === File Info Panel (big discovery improvement) ===
    let info_block = Block::new()
        .title(" File Info ")
        .borders(Borders::ALL)
        .border_style(Color::Cyan);

    let info_text = if let Some(selected_idx) = app.list_state.selected() {
        // Find the corresponding file in the filtered list
        if let Some(path) = filtered_files.get(selected_idx) {
            let meta = std::fs::metadata(path).ok();
            let size = meta
                .as_ref()
                .map(|m| format_file_size(m.len()))
                .unwrap_or_else(|| "Unknown".to_string());
            let modified = meta
                .and_then(|m| m.modified().ok())
                .map(|t| {
                    // Simple formatting
                    format!("{:?}", t)
                        .trim_start_matches("SystemTime { tv_sec: ")
                        .trim_end_matches(" }")
                        .to_string()
                })
                .unwrap_or_else(|| "Unknown".to_string());

            format!(
                "\nPath: {}\n\nSize: {}\nModified: {}\n\nColumns: (load to inspect)\n\nPress Enter to load this file.",
                path.display(),
                size,
                modified
            )
        } else {
            "\nNo file selected".to_string()
        }
    } else {
        "\nSelect a file to see details".to_string()
    };

    let info = Paragraph::new(info_text).block(info_block);
    frame.render_widget(info, list_chunks[1]);

    // === Action bar / key legend (much better UX) ===
    let action_text = if app.filter_mode {
        "  / : Filter   Esc/Enter : Exit filter   ↑↓ : Navigate"
    } else if app.pending_generate {
        "  1/2/3 : Choose preset   Esc : Cancel"
    } else {
        "  / : Filter   Ctrl+G : Generate   c : Convert   Ctrl+R : Refresh   Enter : Load   Ctrl+1/2/3/4, Ctrl+←/→ : Tabs"
    };

    let action_bar = Paragraph::new(action_text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::new().borders(Borders::TOP));

    // Place it at the very bottom of the tab area
    let action_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };
    frame.render_widget(action_bar, action_area);

    // Old filter status (can be removed later)
    let filter_status = if app.file_filter.is_empty() {
        "Filter: (none)".dim()
    } else {
        format!(
            "Filter: \"{}\"  ({} matches)",
            app.file_filter,
            filtered_files.len()
        )
        .into()
    };
    // We can keep a small status in the main area or remove this.
}

/// Render the Explore tab (stats, processing, visualization, edim/fnn)
fn render_explore_tab(frame: &mut Frame, app: &App, area: Rect) {
    if app.pending_process {
        // Processing menu overlay (consistent style with generate menu)
        let names = ["Moving Average", "Median Filter", "Detrend (mean)"];
        let mut lines = vec![
            "\n\nSignal Processing\n\n".to_string(),
            "Choose operation:\n\n".to_string(),
        ];

        for (i, name) in names.iter().enumerate() {
            let prefix = if i == app.process_selection {
                "▶ "
            } else {
                "  "
            };
            if i < 2 {
                lines.push(format!(
                    "{} {}  —  window = {}\n",
                    prefix, name, app.process_window
                ));
            } else {
                lines.push(format!("{} {}\n", prefix, name));
            }
        }

        lines.push(
            "\n\n←/→ or -/+ : Adjust window     Enter : Apply     Esc : Cancel\n".to_string(),
        );

        let content = Paragraph::new(lines.join("")).centered().block(
            Block::new()
                .title(" Processing ")
                .borders(Borders::ALL)
                .border_style(Color::Yellow),
        );

        frame.render_widget(content, area);
        return;
    }

    if let Some(signal) = &app.loaded_signal {
        render_explore_content(frame, app, area, signal);
    } else {
        let placeholder = Paragraph::new(
            "\n\nNo signal loaded yet.\n\n\
             Go to the Import tab (press 1), load a file with Enter,\n\
             then switch back here (press 2) to explore statistics and processing.\n\n\
             Press 'p' for processing controls once a signal is loaded.",
        )
        .centered()
        .block(
            Block::new()
                .title(" Explore — Statistics, Processing & Visualization ")
                .borders(Borders::ALL)
                .border_style(Color::Blue),
        );
        frame.render_widget(placeholder, area);
    }
}

/// Temporary content for the Explore tab (will be expanded with proper layout, filtering, edim/fnn, etc.)
fn render_explore_content(frame: &mut Frame, _app: &App, area: Rect, signal: &LoadedSignal) {
    // Better layout: small header + stats (compact) + big visualization
    let chunks = Layout::vertical([
        Constraint::Length(3), // header (name + length)
        Constraint::Length(6), // stats table
        Constraint::Min(6),    // sparkline visualization
    ])
    .split(area);

    // === Header ===
    let header = Paragraph::new(format!(
        "  {}   •   {} samples (original: {})",
        signal.name,
        signal.n_samples,
        signal.original.len()
    ))
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
    .block(Block::new().borders(Borders::ALL).title(" Loaded Signal "));
    frame.render_widget(header, chunks[0]);

    // === Summary Statistics (compact) ===
    let stats = compute_basic_stats(&signal.current);
    let stats_table = Table::new(
        vec![
            Row::new(vec![
                Cell::from("Mean"),
                Cell::from(format!("{:.4}", stats.mean)),
                Cell::from("Std"),
                Cell::from(format!("{:.4}", stats.std)),
            ]),
            Row::new(vec![
                Cell::from("Min"),
                Cell::from(format!("{:.4}", stats.min)),
                Cell::from("Max"),
                Cell::from(format!("{:.4}", stats.max)),
            ]),
            Row::new(vec![
                Cell::from("Median"),
                Cell::from(format!("{:.4}", stats.median)),
                Cell::from("Length"),
                Cell::from(signal.n_samples.to_string()),
            ]),
        ],
        [
            Constraint::Length(10),
            Constraint::Length(14),
            Constraint::Length(8),
            Constraint::Length(14),
        ],
    )
    .block(
        Block::new()
            .title(" Summary Statistics ")
            .borders(Borders::ALL)
            .border_style(Color::Blue),
    );

    frame.render_widget(stats_table, chunks[1]);

    // === Simple Visualization: Sparkline ===
    // We scale the data into a reasonable u64 range for the sparkline.
    let viz_block = Block::new()
        .title(" Time Series (Sparkline) — full series ")
        .borders(Borders::ALL)
        .border_style(Color::Magenta);

    let viz_inner = viz_block.inner(chunks[2]);
    frame.render_widget(viz_block, chunks[2]);

    if !signal.current.is_empty() {
        let min = stats.min;
        let max = stats.max;
        let range = if max > min { max - min } else { 1.0 };

        // Scale to 0..200 for decent vertical resolution in the sparkline
        let spark_data: Vec<u64> = signal
            .current
            .iter()
            .map(|&v| (((v - min) / range) * 200.0) as u64)
            .collect();

        let sparkline = Sparkline::default()
            .block(Block::new()) // already accounted for the outer block
            .data(&spark_data)
            .style(Style::default().fg(Color::LightCyan))
            .max(200);

        frame.render_widget(sparkline, viz_inner);
    } else {
        let empty = Paragraph::new("(empty series)").centered();
        frame.render_widget(empty, viz_inner);
    }
}

/// Render the Dynamics tab (nonlinear analysis - RQA etc.)
fn render_dynamics_tab(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::new()
        .title(" Nonlinear Dynamics (RQA, Recurrence Plots, etc.) ")
        .borders(Borders::ALL)
        .border_style(Color::Green);

    let content = if app.loaded_signal.is_some() {
        Paragraph::new(
            "\n\nThis tab will host RQA, RecurrencePlot visualization,\n\
             embedding dimension analysis, and related nonlinear tools.\n\n\
             (Implementation planned after Explore tab is solid)",
        )
        .centered()
    } else {
        Paragraph::new("\n\nLoad a signal in the Import tab first.").centered()
    };

    frame.render_widget(content.block(block), area);
}

/// First ratatui sketch for Spatial (wiring SpatialFrame + ground truth + basic viz).
fn render_spatial_tab(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::new()
        .title(" Spatial (synthetic trajectories, decisions, frames) ")
        .borders(Borders::ALL)
        .border_style(Color::Cyan);

    if let (Some(batch), Some(focal)) = (&app.spatial_batch, &app.spatial_focal) {
        let idx = app
            .spatial_frame_idx
            .min(batch.num_times().saturating_sub(1));
        if let Some(spatial_frame) = batch.frame(idx) {
            let mut lines = vec![
                format!(
                    "Frame {} / {}  |  t={:.2}s",
                    idx,
                    batch.num_times().saturating_sub(1),
                    spatial_frame.time
                ),
                format!(
                    "Agents: {}  |  Focal: ({:.1}, {:.1})",
                    spatial_frame.num_agents(),
                    spatial_frame.focal_pos().map_or(0.0, |p| p.x),
                    spatial_frame.focal_pos().map_or(0.0, |p| p.y)
                ),
            ];
            if let Some(labels) = &app.spatial_labels {
                if !labels.is_empty() && idx < labels[0].len() {
                    let g0 = &labels[0][idx];
                    let g1 = if labels.len() > 1 {
                        &labels[1][idx]
                    } else {
                        g0
                    };
                    let g2 = if labels.len() > 2 {
                        &labels[2][idx]
                    } else {
                        g0
                    };
                    lines.push(format!(
                        "Ground truth (agent0,1,2): {:?} {:?} {:?}",
                        g0, g1, g2
                    ));
                }
            }
            // Simple text viz of positions (ratatui sketch - could use Canvas later)
            lines.push("Positions (x,y):".to_string());
            for (i, p) in spatial_frame.agent_positions.iter().enumerate() {
                let label = if let Some(labs) = &app.spatial_labels {
                    if i < labs.len() && idx < labs[i].len() {
                        format!("{:?}", labs[i][idx])
                    } else {
                        "".into()
                    }
                } else {
                    "".into()
                };
                lines.push(format!("  Agent{}: ({:5.1},{:5.1}) {}", i, p.x, p.y, label));
            }
            if idx < focal.len() {
                let fpos = focal[idx];
                lines.push(format!("  Focal : ({:5.1},{:5.1})", fpos.x, fpos.y));
            } else if let Some(fpos) = spatial_frame.focal_pos() {
                lines.push(format!("  Focal : ({:5.1},{:5.1})", fpos.x, fpos.y));
            }

            let content = Paragraph::new(lines.join("\n")).block(block);
            frame.render_widget(content, area);
        } else {
            let content = Paragraph::new("No frame data").block(block);
            frame.render_widget(content, area);
        }
    } else {
        let content =
            Paragraph::new("No spatial data loaded (synthetic demo should have seeded on start)")
                .block(block);
        frame.render_widget(content, area);
    }
}

// render_visualizing removed — replaced by the tab-based render_explore_content + render_import_tab etc.
