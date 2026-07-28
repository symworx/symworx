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
        Padding,
        Paragraph,
    },
    Frame,
};

use crate::app::App;

pub fn render_home_tab(frame: &mut Frame, app: &App, area: Rect) {
    if app.help_mode {
        let help = Paragraph::new(
            "Home — path selector\n\
             Close help:  Esc  or  Alt-?\n\n\
             \n\
             CHOOSE A WORKFLOW\n\n\
               1  or  ↑↓ + Enter     BioSym\n\
                                     Import · Explore · Dynamics · Generate\n\n\
               2  or  ↑↓ + Enter     StatsSym\n\
                                     Import · Lab · Generate\n\n\
               3  or  ↑↓ + Enter     LoadSym\n\
                                     Workout · Metrics · Calendar · Optimization\n\n\
               4  or  ↑↓ + Enter     SpatialSym\n\
                                     Trajectories · decisions · space\n\n\
             \n\
             GLOBAL\n\n\
               Ctrl+H                return Home from anywhere\n\
               Alt-?                 toggle this help\n\
               Esc  Esc              quit (second Esc at a root screen)\n\
               Ctrl+Q                quit anytime\n",
        )
        .block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(" Help — Home "),
        );
        frame.render_widget(help, area);
        return;
    }

    let outer = Block::new()
        .title("")
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1))
        .border_style(Color::Cyan);

    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::vertical([
        Constraint::Length(16), // logo (slightly shorter for 4 options)
        Constraint::Min(0),     // spacer
        Constraint::Length(4),  // 1 BioSym
        Constraint::Length(4),  // 2 StatsSym
        Constraint::Length(4),  // 3 LoadSym
        Constraint::Length(4),  // 4 SpatialSym
    ])
    .split(inner);

    // Large SymWorx built from \/| and similar symbols (no surrounding box)
    #[rustfmt::skip]
    let logo_lines = vec![
        Line::from(Span::styled(r#"   .--.                                                         "#, Style::default() .fg(Color::Magenta) .add_modifier(Modifier::BOLD),)),
        Line::from(Span::styled(r#"  /    \                                                        "#, Style::default() .fg(Color::Magenta) .add_modifier(Modifier::BOLD),)),
        Line::from(Span::styled(r#" |  |\**--.  .--.    .-.             .-.      .--.__.-.    .-.  "#, Style::default() .fg(Color::Magenta) .add_modifier(Modifier::BOLD),)),
        Line::from(Span::styled(r#"  \  \ \   \/   /    \  \           /  /.---. |     \  \  /  /  "#, Style::default() .fg(Color::Magenta) .add_modifier(Modifier::BOLD),)),
        Line::from(Span::styled(r#"   \  \ \      /--.  .--.\         /  //     \|   __|\  \/  /   "#, Style::default() .fg(Color::Magenta) .add_modifier(Modifier::BOLD),)),
        Line::from(Span::styled(r#"    \  \ \    /|   \/   | \  ..   /  /|   .   |  |    \    /    "#, Style::default() .fg(Color::Magenta) .add_modifier(Modifier::BOLD),)),
        Line::from(Span::styled(r#"  .-.|  |/   / |        |  \/  \_/  / |  | |  |  |    /    \    "#, Style::default() .fg(Color::Magenta) .add_modifier(Modifier::BOLD),)),
        Line::from(Span::styled(r#"  \    //   /  |  |\/|  |\   /\    /  |   *   |  |   /  /\  \   "#, Style::default() .fg(Color::Magenta) .add_modifier(Modifier::BOLD),)),
        Line::from(Span::styled(r#"   ****/   /   |  |  |  | ***  ****    \     /***   /  /  \  \  "#, Style::default() .fg(Color::Magenta) .add_modifier(Modifier::BOLD),)),
        Line::from(Span::styled(r#"       ****    ***    ***               *****       ***    ***  "#, Style::default() .fg(Color::Magenta) .add_modifier(Modifier::BOLD),)),
        Line::from(Span::styled(r#"                                                                "#, Style::default() .fg(Color::Magenta) .add_modifier(Modifier::BOLD),)),
        Line::from(Span::styled("        Secure • Robust • Scalable       ", Style::default().fg(Color::Yellow),)),
        Line::from(Span::styled("-----------------------------------------", Style::default().fg(Color::Yellow),)),
        Line::from(Span::styled("          Computational Dynamics         ", Style::default().fg(Color::Yellow),)),
    ];

    let logo = Paragraph::new(logo_lines).alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(logo, chunks[0]);

    // BioSym (1) - selection 0
    let sel = if app.home_selection == 0 { "▶ " } else { "  " };
    let b1 = Paragraph::new(vec![
        Line::from(Span::styled(
            "Biomechanical & Physiological Signals",
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            "Import · Explore · Dynamics · Generate",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .title(format!("{}1 BioSym", sel)),
    );
    if app.home_selection == 0 {
        frame.render_widget(b1.clone().style(Style::default().fg(Color::Cyan)), chunks[2]);
    } else {
        frame.render_widget(b1, chunks[2]);
    }

    // StatsSym (2) - selection 1
    let sel = if app.home_selection == 1 { "▶ " } else { "  " };
    let b2 = Paragraph::new(vec![
        Line::from(Span::styled(
            "Import → Lab · guided stats & demos",
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            "Ctrl+G generate · Ctrl+←→ tabs (students & research)",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .title(format!("{}2 StatsSym", sel)),
    );
    if app.home_selection == 1 {
        frame.render_widget(b2.clone().style(Style::default().fg(Color::Magenta)), chunks[3]);
    } else {
        frame.render_widget(b2, chunks[3]);
    }

    // LoadSym (3) - selection 2
    let sel = if app.home_selection == 2 { "▶ " } else { "  " };
    let b3 = Paragraph::new(vec![
        Line::from(Span::styled(
            "Training Load, ACWR, Monotony & Nutrition",
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            "Workout · Metrics · Calendar · Optimization",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .title(format!("{}3 LoadSym", sel)),
    );
    if app.home_selection == 2 {
        frame.render_widget(b3.clone().style(Style::default().fg(Color::Cyan)), chunks[4]);
    } else {
        frame.render_widget(b3, chunks[4]);
    }

    // SpatialSym (4) - selection 3
    let sel = if app.home_selection == 3 { "▶ " } else { "  " };
    let b4 = Paragraph::new(vec![
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
            .padding(Padding::horizontal(1))
            .title(format!("{}4 SpatialSym", sel)),
    );
    if app.home_selection == 3 {
        frame.render_widget(b4.clone().style(Style::default().fg(Color::Cyan)), chunks[5]);
    } else {
        frame.render_widget(b4, chunks[5]);
    }
}
