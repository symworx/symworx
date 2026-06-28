// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

use anyhow::Result;
use crossterm::event::{self, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{DefaultTerminal, Frame};
use std::time::Duration;

mod app;
mod input;
mod ui;
mod convert;
mod generate;
mod processing;

use app::{App, Tab};

fn main() -> Result<()> {
    color_eyre::install().expect("Failed to install color_eyre");
    ratatui::run(app)?;
    Ok(())
}

fn app(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    let mut app = App::new();
    let tick_rate = Duration::from_millis(100);

    loop {
        terminal.draw(|frame| ui::ui(frame, &mut app))?;

        if event::poll(tick_rate)? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && input::handle_key(&mut app, key.code, key.modifiers) {
                    return Ok(());
                }
            }
        }
    }
}
