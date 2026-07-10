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
    },
    text::{
        Line,
        Span,
    },
    widgets::{
        Block,
        Borders,
        Paragraph,
    },
    Frame,
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
             q: quit • ? (Alt): this help",
        )
        .block(Block::new().borders(Borders::ALL).title(" Help — Home "));
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
        Constraint::Length(20), // large logo at top
        Constraint::Min(0),     // spacer to push selections to bottom
        Constraint::Length(5),  // option 1 (BioSym)
        Constraint::Length(5),  // option 2 (LoadSym)
        Constraint::Length(5),  // option 3 (SpatialSym)
    ])
    .split(inner);

    // Large SymWorx built from \/| and similar symbols (no surrounding box)
    let logo_lines = vec![
        Line::from(Span::styled(
            r#"   .--.                                                         "#,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            r#"  /    \                                                        "#,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            r#" |  |\**--.  .--.    .-.             .-.      .--.__.-.    .-.  "#,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            r#"  \  \ \   \/   /    \  \           /  /.---. |     \  \  /  /  "#,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            r#"   \  \ \      /--.  .--.\         /  //     \|   __|\  \/  /   "#,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            r#"    \  \ \    /|   \/   | \  ..   /  /|   *   |  |    \    /    "#,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            r#"  .-.|  |/   / |        |  \/  \_/  / |  | |  |  |    /    \    "#,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            r#"  \    //   /  |  |\/|  |\   /\    /  |   *   |  |   /  /\  \   "#,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            r#"   ****/   /   |  |  |  | ***  ****    \     /***   /  /  \  \  "#,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            r#"       ****    ***    ***               *****       ***    ***  "#,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            r#"                                                                "#,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "        Secure • Robust • Scalable       ",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "-----------------------------------------",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "          Computational Dynamics         ",
            Style::default().fg(Color::Yellow),
        )),
    ];

    let logo = Paragraph::new(logo_lines).alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(logo, chunks[0]);

    // BioSym (1) - selection 0
    let sel = if app.home_selection == 0 {
        "▶ "
    } else {
        "  "
    };
    let b1 = Paragraph::new(vec![
        Line::from(Span::styled(
            "Biomechanical & Physiological Signals",
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            "Import • Explore • RQA + nonlinear dynamics",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::new()
            .borders(Borders::ALL)
            .title(format!("{}1 BioSym", sel)),
    );
    if app.home_selection == 0 {
        frame.render_widget(
            b1.clone().style(Style::default().fg(Color::Cyan)),
            chunks[2],
        );
    } else {
        frame.render_widget(b1, chunks[2]);
    }

    // LoadSym (2) - selection 1
    let sel = if app.home_selection == 1 {
        "▶ "
    } else {
        "  "
    };
    let b2 = Paragraph::new(vec![
        Line::from(Span::styled(
            "Training Load, ACWR, Monotony & Nutrition",
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            "Workout analysis • Calendar • Programming optimization",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::new()
            .borders(Borders::ALL)
            .title(format!("{}2 LoadSym", sel)),
    );
    if app.home_selection == 1 {
        frame.render_widget(
            b2.clone().style(Style::default().fg(Color::Cyan)),
            chunks[3],
        );
    } else {
        frame.render_widget(b2, chunks[3]);
    }

    // SpatialSym (3) - selection 2
    let sel = if app.home_selection == 2 {
        "▶ "
    } else {
        "  "
    };
    let b3 = Paragraph::new(vec![
        Line::from(Span::styled(
            "Trajectory & Spatial Decision Analysis",
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            "Synthetic • Import matches • Frame viz + actions",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::new()
            .borders(Borders::ALL)
            .title(format!("{}3 SpatialSym", sel)),
    );
    if app.home_selection == 2 {
        frame.render_widget(
            b3.clone().style(Style::default().fg(Color::Cyan)),
            chunks[4],
        );
    } else {
        frame.render_widget(b3, chunks[4]);
    }
    // Note: bottom navigation hints removed to avoid duplication with top chrome arrows
}
