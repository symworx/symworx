// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

use ratatui::{
    layout::Rect,
    widgets::{
        Block,
        Borders,
        Padding,
        Paragraph,
    },
    Frame,
};

pub fn render_placeholder(frame: &mut Frame, area: Rect, title: &str, body: &str) {
    frame.render_widget(
        Paragraph::new(body).block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(title),
        ),
        area,
    );
}
