use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use crate::app::App;

pub fn render_home_tab(frame: &mut Frame, app: &App, area: Rect) {
    if app.help_mode {
        let help = Paragraph::new(
            "Home / Path Selector help (Alt-? or Esc to close)\n\n\
             • 1 or ↑/↓+Enter : BioSym (Import → Explore → Dynamics RQA)\n\
             • 2 : LoadSym (training load / ACWR / nutrition template)\n\
             • 3 : SpatialSym (synthetic trajectories + import + viz)\n\n\
             From anywhere: Ctrl+H returns here.\n\
             q: quit • ? (Alt): this help"
        ).block(Block::new().borders(Borders::ALL).title(" Help — Home "));
        frame.render_widget(help, area);
        return;
    }

    let outer = Block::new()
        .title("")
        .borders(Borders::ALL)
        .border_style(Color::Cyan);

    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(6),
        Constraint::Length(6),
        Constraint::Length(6),
    ])
    .split(inner);

    let header = Paragraph::new(vec![
        Line::from(Span::styled("This TUI is part of the SymWorx project.", Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
        // Line::from(Span::styled("Use ↑↓ or number keys to select, Enter to activate. Ctrl+H always returns here.", Style::default().fg(Color::DarkGray))),
    ]);
    frame.render_widget(header, chunks[0]);

    // BioSym (1) - selection 0
    let sel = if app.home_selection == 0 { "▶ " } else { "  " };
    let b1 = Paragraph::new(vec![
        Line::from(format!("{}1. BioSym / Biomechanical and Physiological Simulation and Analysis", sel)),
        Line::from(Span::styled("   Import (PPG, respiration, stride, IBI, CSV) • Explore stats + basic filters • Dynamics: RQA + recurrence plots", Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled("   (signal filtering and nonlinear analysis of biomechanical and physiological signals.)", Style::default().fg(Color::DarkGray))),
    ]).block(Block::new().borders(Borders::ALL).title(" BioSym "));
    if app.home_selection == 0 {
        frame.render_widget(b1.clone().style(Style::default().fg(Color::Cyan)), chunks[1]);
    } else {
        frame.render_widget(b1, chunks[1]);
    }

    // LoadSym (2) - selection 1
    let sel = if app.home_selection == 1 { "▶ " } else { "  " };
    let b2 = Paragraph::new(vec![
        Line::from(format!("{}2. LoadSym", sel)),
        Line::from(Span::styled("   Training load (ACWR, mechanical load, monotony) + nutrition analysis", Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled("   (Empty template — future loadsym integration)", Style::default().fg(Color::DarkGray))),
    ]).block(Block::new().borders(Borders::ALL).title(" LoadSym "));
    if app.home_selection == 1 {
        frame.render_widget(b2.clone().style(Style::default().fg(Color::Cyan)), chunks[2]);
    } else {
        frame.render_widget(b2, chunks[2]);
    }

    // SpatialSym (3) - selection 2
    let sel = if app.home_selection == 2 { "▶ " } else { "  " };
    let b3 = Paragraph::new(vec![
        Line::from(format!("{}3. SpatialSym / Spatial Data Trajectory & Decision Analysis", sel)),
        Line::from(Span::styled("   Synthetic generators + import trajectory CSVs (time/agent/x/y) • Frame viz • Carrier inference • SpaceAction classification", Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled("   (expansion / penetration / denial / pressure)", Style::default().fg(Color::DarkGray))),
    ]).block(Block::new().borders(Borders::ALL).title(" SpatialSym "));
    if app.home_selection == 2 {
        frame.render_widget(b3.clone().style(Style::default().fg(Color::Cyan)), chunks[3]);
    } else {
        frame.render_widget(b3, chunks[3]);
    }
    // Note: bottom navigation hints removed to avoid duplication with top chrome arrows
}
