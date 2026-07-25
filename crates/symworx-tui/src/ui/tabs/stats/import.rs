// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

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
    widgets::{
        Block,
        Borders,
        List,
        ListItem,
        Padding,
        Paragraph,
    },
    Frame,
};

use crate::app::App;

pub fn render_import(frame: &mut Frame, app: &mut App, area: Rect) {
    let show_header = app.filter_mode || !app.file_filter.is_empty() || !app.manual_path.is_empty();

    let chunks = Layout::vertical([
        Constraint::Length(if show_header { 2 } else { 1 }),
        Constraint::Min(5),
        Constraint::Length(if app.stats_table.is_some() { 6 } else { 0 }),
        Constraint::Length(1),
    ])
    .split(area);

    // Hint / filter / path line
    let mut h = String::from("Ctrl+G generate  ·  Enter load → Lab  ·  / filter");
    if app.filter_mode || !app.file_filter.is_empty() {
        h = format!("Filter: {}  ", app.file_filter);
    }
    if !app.manual_path.is_empty() {
        h.push_str(&format!("  Manual: {}", app.manual_path));
    }
    frame.render_widget(
        Paragraph::new(h).style(Style::default().fg(Color::Yellow)),
        chunks[0],
    );

    // File list (shared discovery with BioSym)
    let vis = app.visible_indices();
    let title = if !app.file_filter.is_empty() {
        format!(
            " Import ({} / {} matching '{}') ",
            vis.len(),
            app.file_list.len(),
            app.file_filter
        )
    } else {
        " Import (file discovery · numeric CSV) ".to_string()
    };

    let items: Vec<ListItem> = vis
        .iter()
        .map(|&orig| ListItem::new(app.file_list[orig].display().to_string()))
        .collect();

    let list = List::new(items)
        .block(
            Block::new()
                .title(title)
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .border_style(Color::Magenta),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, chunks[1], &mut app.list_state);

    // Compact last-table summary (if any) — full analysis is on Lab
    if let Some(ref t) = app.stats_table {
        if chunks[2].height > 0 {
            let names: String = t
                .headers
                .iter()
                .enumerate()
                .map(|(i, h)| format!("[{i}]{h}"))
                .collect::<Vec<_>>()
                .join("  ");
            let body = format!(
                "Last table: {}×{}  ({})\n{}\nCtrl+→ Lab to analyze",
                t.n_rows(),
                t.n_cols(),
                t.source,
                names
            );
            frame.render_widget(
                Paragraph::new(body).block(
                    Block::new()
                        .borders(Borders::ALL)
                        .padding(Padding::horizontal(1))
                        .title(" Loaded columns "),
                ),
                chunks[2],
            );
        }
    }

    frame.render_widget(
        Paragraph::new("↑↓ select  Enter load→Lab  x delete  / filter  type path  Ctrl+G generate")
            .style(Style::default().fg(Color::DarkGray)),
        chunks[3],
    );
}
