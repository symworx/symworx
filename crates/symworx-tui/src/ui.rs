use ratatui::{
    layout::{
        Alignment,
        Constraint,
        Layout,
    },
    style::{
        Color,
        Modifier,
        Style,
        Stylize,
    },
    widgets::{
        Block,
        Borders,
        Paragraph,
    },
    Frame,
};

use crate::app::{
    App,
    Tab,
};

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
        Constraint::Length(9), // left: SymView
        Constraint::Fill(1),   // center: primary name takes flexible space → dynamic centering
        Constraint::Min(12),   // right: subtab/HOME
    ])
    .split(header_area);

    // Left: always SymView
    let left = Paragraph::new("SymView").style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );
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
    let right_p = Paragraph::new(right).alignment(Alignment::Right).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(right_p, header_chunks[2]);

    let action_bar = render_action_bar(app);
    frame.render_widget(action_bar, main_layout[1]);

    // When workflow is Home (or tab explicitly Home) render the landing full-height
    if is_home(app) {
        tabs::home::render_home_tab(frame, app, main_layout[2]);
    } else if app.current_workflow == crate::app::Workflow::LoadSym
        || app.current_tab == Tab::LoadSym
    {
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

    // Lower legend / footer bar:
    // - Left: status + [Esc] when relevant (for subdir/analysis pages / sub-modes)
    // - Right: Ctrl+Q Quit (right-aligned)
    // No tab name here (shown in top-right chrome)
    let footer_area = main_layout[3];
    let footer_chunks = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Min(18), // room for "Esc Esc · Ctrl+Q"
    ])
    .split(footer_area);

    // Append [Esc] hint when entering sub-pages / analysis modes where Escape is relevant
    let esc_hint = if app.pending_process
        || app.pending_peak_params
        || app.pending_rqa
        || app.pending_spatial_import
        || app.filter_mode
    {
        "  [Esc] cancel"
    } else if app.is_live() {
        "  [Esc] stop live"
    } else if app.current_workflow == crate::app::Workflow::LoadSym
        && app.loadsym_view != crate::app::LoadSymView::List
    {
        "  [Esc] back"
    } else if app.current_workflow == crate::app::Workflow::SpatialSym && app.pending_spatial_import
    {
        "  [Esc] back"
    } else if app.pending_generate {
        "  [Esc] cancel"
    } else if app.current_workflow == crate::app::Workflow::BioSym
        && app.current_tab == crate::app::Tab::Explore
    {
        "  [Esc] Import"
    } else {
        ""
    };

    let left_text = if esc_hint.is_empty() {
        format!(" {}", app.status)
    } else {
        format!(" {}{}", app.status, esc_hint)
    };

    let left_legend = Paragraph::new(left_text).dim();

    let right_quit = Paragraph::new(if app.esc_quit_pending {
        "Esc again quit"
    } else {
        "Esc Esc · Ctrl+Q"
    })
    .alignment(Alignment::Right)
    .dim();

    // Single top border across the whole bottom area
    let border_block = Block::new().borders(Borders::TOP);
    frame.render_widget(border_block, footer_area);

    frame.render_widget(left_legend, footer_chunks[0]);
    frame.render_widget(right_quit, footer_chunks[1]);
}

pub fn render_action_bar(app: &App) -> Paragraph<'_> {
    // Lower legend / action bar.
    // Show [Esc] when relevant for sub-modes / analysis pages (back/cancel).
    // Ctrl+Q Quit is right-aligned in the footer below.

    if app.pending_generate {
        return Paragraph::new("  BioSym: [1] PPG   [2] Respiration   [3] Stride   [Esc] Cancel")
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
    }
    if app.filter_mode {
        return Paragraph::new("  Filtering...  [Esc] or [Enter] to exit")
            .style(Style::default().fg(Color::Cyan));
    }

    // Determine if we are in a "subdir/analysis page" where Esc is relevant
    let esc = if app.pending_process {
        "  [Esc] cancel process"
    } else if app.pending_peak_params {
        "  [Esc] close peak params"
    } else if app.pending_rqa {
        "  [Esc] cancel RQA"
    } else if app.pending_spatial_import {
        "  [Esc] cancel import"
    } else if app.is_live() {
        "  [Esc] stop live"
    } else if app.current_workflow == crate::app::Workflow::LoadSym {
        if app.pending_workout_open {
            "  [Esc] cancel open"
        } else {
            match app.loadsym_view {
                crate::app::LoadSymView::List => "",
                _ => "  [Esc] back to list",
            }
        }
    } else if app.current_workflow == crate::app::Workflow::SpatialSym && app.pending_spatial_import
    {
        "  [Esc] back"
    } else if app.current_workflow == crate::app::Workflow::BioSym
        && app.current_tab == crate::app::Tab::Explore
    {
        "  [Esc] Import"
    } else {
        ""
    };

    let base = if app.is_live() {
        "  Ctrl+L restart live   •   Ctrl+H Home   •   Alt-? help   •   Esc Esc / Ctrl+Q quit"
    } else if app.current_workflow == crate::app::Workflow::BioSym {
        "  ↑↓   ←→ (Ctrl+arrows)   •   Enter   •   Ctrl+L live   •   Ctrl+H Home   •   Alt-? help"
    } else {
        "  ↑↓   ←→ (Ctrl+arrows)   •   Enter   •   Ctrl+H Home   •   Alt-? help"
    };

    let text = if esc.is_empty() {
        base.to_string()
    } else {
        format!("{}{}", esc, base)
    };

    Paragraph::new(text).style(Style::default().fg(Color::DarkGray))
}
