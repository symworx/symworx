use ratatui::{
    Frame,
    layout::{Constraint, Layout, Alignment},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, Borders, Paragraph},
};
use crate::app::{App, Tab};

pub mod tabs;

fn is_home(app: &App) -> bool {
    app.current_workflow == crate::app::Workflow::Home || app.current_tab == Tab::Home
}

pub fn ui(frame: &mut Frame, app: &mut App) {
    let main_layout = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(8),
        Constraint::Length(2),
    ])
    .split(frame.area());

    // Unified top chrome:
    //   Left: "SymView" (fixed)
    //   Center: key/primary tab name (e.g. "BioSym"), *dynamically centered* in remaining space
    //   Right: current subtab or "HOME" (right-aligned)
    let header_area = main_layout[0];
    let header_chunks = Layout::horizontal([
        Constraint::Length(9),   // left: SymView
        Constraint::Fill(1),     // center: primary name takes flexible space → dynamic centering
        Constraint::Min(12),     // right: subtab/HOME
    ])
    .split(header_area);

    // Left: always SymView
    let left = Paragraph::new("SymView")
        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    frame.render_widget(left, header_chunks[0]);

    // Center: primary (parent) tab name — centered within the flexible center area
    let parent = match app.current_workflow {
        crate::app::Workflow::Home => "",
        crate::app::Workflow::BioSym => "BioSym",
        crate::app::Workflow::SpatialSym => "SpatialSym",
        crate::app::Workflow::LoadSym => "LoadSym",
    };
    let center = Paragraph::new(parent)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(center, header_chunks[1]);

    // Right: HOME or subtab name, right aligned
    let right = if is_home(app) {
        "HOME".to_string()
    } else {
        match app.current_workflow {
            crate::app::Workflow::BioSym => app.current_tab.title().to_string(),
            crate::app::Workflow::SpatialSym => "Spatial".to_string(),
            crate::app::Workflow::LoadSym => "LoadSym".to_string(),
            _ => app.current_tab.title().to_string(),
        }
    };
    let right_p = Paragraph::new(right)
        .alignment(Alignment::Right)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    frame.render_widget(right_p, header_chunks[2]);

    let action_bar = render_action_bar(app);
    frame.render_widget(action_bar, main_layout[1]);

    // When workflow is Home (or tab explicitly Home) render the landing full-height
    if is_home(app) {
        tabs::home::render_home_tab(frame, app, main_layout[2]);
    } else if app.current_workflow == crate::app::Workflow::LoadSym || app.current_tab == Tab::LoadSym {
        tabs::loadsym::render_loadsym_tab(frame, app, main_layout[2]);
    } else {
        match app.current_tab {
            Tab::Import => tabs::import::render_import_tab(frame, app, main_layout[2]),
            Tab::Explore => tabs::explore::render_explore_tab(frame, app, main_layout[2]),
            Tab::Dynamics => tabs::dynamics::render_dynamics_tab(frame, app, main_layout[2]),
            Tab::Spatial => tabs::spatial::render_spatial_tab(frame, app, main_layout[2]),
            Tab::LoadSym => tabs::loadsym::render_loadsym_tab(frame, app, main_layout[2]),
            Tab::Home => tabs::home::render_home_tab(frame, app, main_layout[2]),
        }
    }

    let footer = Paragraph::new(format!(" {}  •  q: Quit", app.status))
        .centered()
        .dim()
        .block(Block::new().borders(Borders::TOP));
    frame.render_widget(footer, main_layout[3]);
}

pub fn render_action_bar(app: &App) -> Paragraph<'_> {
    // Simplified: only movement related commands (arrows) + universal navigation.
    // Detailed commands removed from chrome (see status line or inside views).
    if app.pending_generate {
        return Paragraph::new("  BioSym: [1] PPG   [2] Respiration   [3] Stride   [Esc] Cancel")
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    }
    if app.filter_mode {
        return Paragraph::new("  Filtering...  [Esc] or [Enter] to exit")
            .style(Style::default().fg(Color::Cyan));
    }

    Paragraph::new("  ↑↓   ←→ (Ctrl+arrows)   •   Enter   •   Ctrl+H Home   •   M-? help")
        .style(Style::default().fg(Color::DarkGray))
}
