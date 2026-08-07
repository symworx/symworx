// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

use ratatui::{
    Frame,
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
};

use crate::app::App;

pub fn render_generate(frame: &mut Frame, app: &App, area: Rect) {
    use symworx_stats::SyntheticPreset;

    let presets = SyntheticPreset::ALL;
    let sel = app.stats_gen_preset.min(presets.len().saturating_sub(1));
    let p = presets[sel];

    let chunks = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(8),
        Constraint::Length(4),
        Constraint::Length(1),
    ])
    .split(area);

    let hdr = Paragraph::new(vec![
        Line::from(Span::styled(
            "Teaching presets (reproducible seed)",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(format!(
            "n={}  seed={}  noise={:.2}  ·  ↑↓ preset  n/N size  s/S seed  +/− noise  Enter run",
            app.stats_gen_n, app.stats_gen_seed, app.stats_gen_noise
        )),
        Line::from(Span::styled(p.description(), Style::default().fg(Color::Yellow))),
    ])
    .block(
        Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .title(" 3. Generate "),
    );
    frame.render_widget(hdr, chunks[0]);

    let mut lines = vec![Line::from(Span::styled(
        "  PRESET",
        Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
    ))];
    for (i, pr) in presets.iter().enumerate() {
        let marker = if i == sel { "▶" } else { " " };
        let style = if i == sel {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Magenta)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(
            format!("{marker} {}. {}", i + 1, pr.label()),
            style,
        )));
    }
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(" Presets "),
        ),
        chunks[1],
    );

    let notes = if app.stats_gen_notes.is_empty() {
        "Ground-truth notes appear here after Enter.".to_string()
    } else {
        app.stats_gen_notes.clone()
    };
    frame.render_widget(
        Paragraph::new(notes).block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(" Notes "),
        ),
        chunks[2],
    );
    frame.render_widget(
        Paragraph::new("Enter generate → Lab  ·  Esc Import  ·  Ctrl+←→ tabs")
            .style(Style::default().fg(Color::DarkGray)),
        chunks[3],
    );
}
