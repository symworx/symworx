// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! BioSym Generate tab — synthetic biosignal demos (Ctrl+G).

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

use crate::{
    app::App,
    generate::DemoPreset,
};

pub fn render_bio_generate_tab(frame: &mut Frame, app: &App, area: Rect) {
    if app.help_mode {
        let help = Paragraph::new(
            "BioSym — Generate demo signals\n\
             Close help:  Esc  or  Alt-?\n\n\
             \n\
             PRESETS\n\n\
               ↑ ↓                 select preset\n\
               1 / 2 / 3           jump + generate immediately\n\
               Enter               generate selected → Explore\n\
               Esc                 back to Import\n\
               Ctrl+←→             Import · Explore · Dynamics · Generate\n\n\
             Writes CSV (+ peaks sidecar when applicable) under ./data/\n\
             then loads the signal into Explore (same as Import load).\n",
        )
        .block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(" Help — BioSym Generate "),
        );
        frame.render_widget(help, area);
        return;
    }

    let presets = DemoPreset::MENU;
    let sel = app.bio_gen_preset.min(presets.len().saturating_sub(1));
    let p = presets[sel];

    let chunks = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(8),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .split(area);

    let hdr = Paragraph::new(vec![
        Line::from(Span::styled(
            "Synthetic biosignals (symworx-biosym)",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(p.description(), Style::default().fg(Color::Yellow))),
        Line::from(Span::styled(
            "↑↓ select  ·  Enter generate → Explore  ·  1/2/3 quick  ·  Esc Import",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .title(" BioSym — Generate ")
            .border_style(Color::Blue),
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
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(
            format!("{marker} {}. {}", i + 1, pr.name()),
            style,
        )));
        lines.push(Line::from(Span::styled(
            format!("     {}", pr.description()),
            Style::default().fg(Color::DarkGray),
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

    frame.render_widget(
        Paragraph::new(
            "Files land in ./data/ with headers. Peaks sidecar *.peaks.csv when applicable.\n\
             After generate you jump to Explore (waveform / peaks / tachogram).",
        )
        .block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(" Notes "),
        ),
        chunks[2],
    );

    frame.render_widget(
        Paragraph::new("Enter generate → Explore  ·  Esc Import  ·  Ctrl+←→ tabs")
            .style(Style::default().fg(Color::DarkGray)),
        chunks[3],
    );
}
