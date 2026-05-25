// Copyright (C) 2026 cSYMd, All rights reserved.

//! symworx-tui — Terminal interface for selecting and visualizing SymWorx data! 
//!
//! This is the initial v0.1 focused on **file selection** for biosym signals.
//! Next steps: integrate real signal loading from your Symworx crates + interactive Chart.

use anyhow::Result;
use crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    DefaultTerminal, Frame,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

/// Main application state
struct App {
    /// Current screen
    screen: Screen,
    /// List of discoverable signal files in the data directory (or current dir)
    file_list: Vec<PathBuf>,
    /// Which file is highlighted in the list
    list_state: ListState,
    /// Manual path input (user can type a full path)
    manual_path: String,
    /// Status / error messages
    status: String,
}

/// The two main screens for v0.1
enum Screen {
    /// File selection / browser
    FileSelect,
    /// Visualization view (stub for now)
    Visualizing {
        path: PathBuf,
        /// Placeholder — replace with real sample count / metadata from your loader
        samples: usize,
    },
}

impl App {
    fn new() -> Self {
        let mut app = Self {
            screen: Screen::FileSelect,
            file_list: Vec::new(),
            list_state: ListState::default(),
            manual_path: String::new(),
            status: "Select a biosym signal file to visualize".to_string(),
        };
        app.refresh_file_list();
        if !app.file_list.is_empty() {
            app.list_state.select(Some(0));
        }
        app
    }

    /// Scan for likely biosignal files.
    /// TODO: Replace/extend this with discovery logic from your Symworx IO or biosym format.
    fn refresh_file_list(&mut self) {
        self.file_list.clear();

        // Look in ./data first, then current directory
        let candidates = ["./data", "."];

        for dir in candidates {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            // Common biosignal / physiological file extensions
                            if matches!(ext.to_lowercase().as_str(), "csv" | "txt" | "dat" | "bin" | "biosym") {
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

        // TODO: Replace this stub with your real biosym / Symworx signal loader.
        // Example future integration:
        // let signal = symworx_signal::load_biosym(&path)?;
        // let samples = signal.len();

        // For v0.1 we just count lines as a fake "sample count"
        let samples = fs::read_to_string(&path)
            .map(|s| s.lines().count())
            .unwrap_or(0);

        self.screen = Screen::Visualizing {
            path: path.clone(),
            samples,
        };
        self.status = format!("Loaded: {}", path.display());
        self.manual_path.clear();
        Ok(())
    }

    fn go_back_to_select(&mut self) {
        self.screen = Screen::FileSelect;
        self.status = "Select another file or press q to quit".to_string();
        self.refresh_file_list();
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
                if key.kind == KeyEventKind::Press {
                    if handle_key(&mut app, key.code) {
                        return Ok(()); // quit requested
                    }
                }
            }
        }
    }
}

/// Returns true if we should quit
fn handle_key(app: &mut App, code: KeyCode) -> bool {
    match app.screen {
        Screen::FileSelect => handle_file_select_keys(app, code),
        Screen::Visualizing { .. } => handle_viz_keys(app, code),
    }
}

fn handle_file_select_keys(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => return true,

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

        // Manual path input
        KeyCode::Char(c) if c.is_ascii() => {
            app.manual_path.push(c);
        }
        KeyCode::Backspace => {
            app.manual_path.pop();
        }

        // Load action
        KeyCode::Enter => {
            if let Err(e) = app.load_selected_or_manual() {
                app.status = format!("Error loading file: {e}");
            }
        }

        // Refresh file list
        KeyCode::Char('r') | KeyCode::F(5) => {
            app.refresh_file_list();
            app.status = "File list refreshed".to_string();
        }

        _ => {}
    }
    false
}

fn handle_viz_keys(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => return true,
        KeyCode::Char('b') | KeyCode::Backspace => {
            app.go_back_to_select();
        }
        _ => {}
    }
    false
}

fn ui(frame: &mut Frame, app: &App) {
    let main_layout = Layout::vertical([
        Constraint::Length(3),  // header
        Constraint::Min(10),    // main content
        Constraint::Length(3),  // footer / status
    ])
    .split(frame.area());

    // Header
    let header = Paragraph::new(" symview — Biosym Signal Visualizer (Symworx) ")
        .bold()
        .centered()
        .block(Block::new().borders(Borders::BOTTOM).border_style(Color::Cyan));
    frame.render_widget(header, main_layout[0]);

    match &app.screen {
        Screen::FileSelect => render_file_select(frame, app, main_layout[1]),
        Screen::Visualizing { path, samples } => {
            render_visualizing(frame, app, main_layout[1], path, *samples)
        }
    }

    // Footer / status
    let footer = Paragraph::new(format!(" {}  |  q: Quit  •  r: Refresh list  •  Enter: Load  •  b: Back ", app.status))
        .centered()
        .dim()
        .block(Block::new().borders(Borders::TOP));
    frame.render_widget(footer, main_layout[2]);
}

fn render_file_select(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(4),   // instructions + manual input
        Constraint::Min(5),      // file list
    ])
    .split(area);

    // Instructions + manual path input
    let input_block = Block::new()
        .title(" Type full path or select from list below (↑↓ / jk, Enter to load) ")
        .borders(Borders::ALL)
        .border_style(Color::Yellow);

    let input_text = if app.manual_path.is_empty() {
        "→  (start typing a path or use the list below)".dim().to_string()
    } else {
        format!("→  {}", app.manual_path)
    };

    let input_para = Paragraph::new(input_text)
        .block(input_block);
    frame.render_widget(input_para, chunks[0]);

    // File list
    let list_block = Block::new()
        .title(format!(" Available signal files ({}) — r to refresh ", app.file_list.len()))
        .borders(Borders::ALL);

    let items: Vec<ListItem> = app
        .file_list
        .iter()
        .map(|p| {
            let name = p.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("???");
            ListItem::new(name.to_string())
        })
        .collect();

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, chunks[1], &mut app.list_state.clone());
}

fn render_visualizing(
    frame: &mut Frame,
    _app: &App,
    area: ratatui::layout::Rect,
    path: &Path,
    samples: usize,
) {
    let chunks = Layout::vertical([
        Constraint::Length(6),  // info panel
        Constraint::Min(5),     // chart area (stub)
    ])
    .split(area);

    // Info panel
    let info = Paragraph::new(format!(
        "\n  File: {}\n  Samples (approx): {}\n\n  TODO: Replace with real metadata from your biosym loader\n  (duration, sampling rate, channels, etc.)",
        path.display(),
        samples
    ))
    .block(
        Block::new()
            .title(" Signal Loaded — Press 'b' to go back ")
            .borders(Borders::ALL)
            .border_style(Color::Green),
    );
    frame.render_widget(info, chunks[0]);

    // Chart placeholder area
    let chart_block = Block::new()
        .title(" Signal Visualization (coming next: real Chart + your Symworx processing) ")
        .borders(Borders::ALL)
        .border_style(Color::Blue);

    let placeholder = Paragraph::new(
        "\n\n[ Interactive Chart will appear here ]\n\n\
         • Zoom / pan with keyboard\n\
         • Overlay filtered signal, peaks, RQA features, etc.\n\
         • Use your existing symworx-signal / peak-detection crates here"
    )
    .centered()
    .block(chart_block);

    frame.render_widget(placeholder, chunks[1]);
}
