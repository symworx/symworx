// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Two panes and a footer. Drawing does not touch the database.

use crossterm::event::{
    self,
    Event,
    KeyEventKind,
};
use ratatui::{
    Frame,
    layout::{
        Constraint,
        Layout,
    },
    style::{
        Color,
        Modifier,
        Style,
    },
    widgets::{
        Block,
        Borders,
        Cell,
        List,
        ListItem,
        ListState,
        Padding,
        Paragraph,
        Row,
        Table,
    },
};

use crate::{
    app::{
        Action,
        App,
        Focus,
    },
    error::Result,
    model::{
        PAGE_SIZE,
        truncate_chars,
    },
    source::CatalogSource,
};

const CELL_WIDTH: usize = 18;

/// Load the relation list, then run until quit. Restores the terminal on the way out.
pub fn run(source: &mut dyn CatalogSource) -> Result<()> {
    let mut app = App::new(source.label().to_string());
    refresh_relations(source, &mut app);
    match source.catalog_hint() {
        Ok(hint) => app.set_hint(hint),
        Err(err) => app.set_status(err.to_string()),
    }
    ratatui::run(|terminal| -> std::io::Result<()> {
        loop {
            terminal.draw(|frame| render(frame, &app))?;
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                continue;
            }
            match app.handle_key(key.code, key.modifiers) {
                Action::Quit => return Ok(()),
                Action::None => {}
                Action::ReloadRelations => refresh_relations(source, &mut app),
                Action::LoadPage { offset } => load_page(source, &mut app, offset),
            }
        }
    })
    .map_err(Into::into)
}

fn refresh_relations(source: &mut dyn CatalogSource, app: &mut App) {
    match source.relations() {
        Ok(relations) => app.set_relations(relations),
        Err(err) => app.set_status(err.to_string()),
    }
}

fn load_page(source: &mut dyn CatalogSource, app: &mut App, offset: u32) {
    let Some(relation) = app.selected_relation().cloned() else {
        return;
    };
    match source.page(&relation, PAGE_SIZE, offset) {
        Ok(page) => app.set_page(page),
        Err(err) => app.set_status(err.to_string()),
    }
}

fn render(frame: &mut Frame, app: &App) {
    if app.help() {
        render_help(frame);
        return;
    }
    let outer = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).split(frame.area());
    let panes = Layout::horizontal([Constraint::Length(28), Constraint::Min(10)]).split(outer[0]);
    render_relations(frame, app, panes[0]);
    render_rows(frame, app, panes[1]);
    frame.render_widget(Paragraph::new(footer(app)), outer[1]);
}

fn render_help(frame: &mut Frame) {
    let body = "\
symdb-view — read-only catalog\n\
Close help:  Esc  or  Alt-?\n\
\n\
\n\
RELATIONS\n\
\n\
  j k   or  ↑ ↓          move\n\
  Enter                  open the table (first page)\n\
  /                      filter names\n\
  Esc                    clear the filter\n\
  r                      reload tables\n\
\n\
\n\
ROWS\n\
\n\
  j k   or  ↑ ↓          move row\n\
  h l   or  ← →          move column\n\
  PgUp  PgDn             page\n\
  Esc                    back to the table list\n\
\n\
\n\
QUIT\n\
\n\
  Esc at the table list  arm quit, Esc again to exit\n\
  Ctrl-Q                 quit now\n\
\n\
The session cannot write. This is not symview and not LoadSym.\n";
    let block = Block::new()
        .title("Help")
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1));
    frame.render_widget(Paragraph::new(body).block(block), frame.area());
}

fn render_relations(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let labels = app.visible_labels();
    let label_count = labels.len();
    let title = if app.filter_mode() {
        format!("Tables  /{}_", app.filter())
    } else if app.filter().is_empty() {
        "Tables".to_string()
    } else {
        format!("Tables  /{}", app.filter())
    };
    let items: Vec<ListItem> = labels.into_iter().map(ListItem::new).collect();
    let list = List::new(items)
        .block(pane_block(title, app.focus() == Focus::Relations))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
    let mut state = ListState::default();
    if label_count == 0 {
        state.select(None);
    } else {
        state.select(Some(app.relation_index()));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_rows(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let title = match app.selected_relation() {
        Some(relation) => format!("{}  read-only", relation.label()),
        None => "Rows  read-only".to_string(),
    };
    let Some(page) = app.page() else {
        let paragraph =
            Paragraph::new("Enter opens the selected table").block(pane_block(title, app.focus() == Focus::Rows));
        frame.render_widget(paragraph, area);
        return;
    };
    if page.columns.is_empty() {
        let paragraph =
            Paragraph::new("This table has no columns").block(pane_block(title, app.focus() == Focus::Rows));
        frame.render_widget(paragraph, area);
        return;
    }
    let start = app.col_cursor().min(page.columns.len().saturating_sub(1));
    let columns = &page.columns[start..];
    let header = Row::new(columns.iter().map(|column| {
        Cell::from(truncate_chars(&column.name, CELL_WIDTH))
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    }));
    let rows: Vec<Row> = page
        .rows
        .iter()
        .enumerate()
        .map(|(index, cells)| {
            let shown = cells
                .iter()
                .skip(start)
                .map(|cell| Cell::from(truncate_chars(cell, CELL_WIDTH)))
                .collect::<Vec<_>>();
            let row = Row::new(shown);
            if index == app.row_cursor() && app.focus() == Focus::Rows {
                row.style(Style::default().add_modifier(Modifier::REVERSED))
            } else {
                row
            }
        })
        .collect();
    let widths = vec![Constraint::Length(CELL_WIDTH as u16); columns.len()];
    let table = Table::new(rows, widths)
        .header(header)
        .block(pane_block(title, app.focus() == Focus::Rows));
    frame.render_widget(table, area);
}

fn pane_block(title: String, focused: bool) -> Block<'static> {
    let border = if focused { Color::Cyan } else { Color::DarkGray };
    Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border)
        .padding(Padding::horizontal(1))
}

fn footer(app: &App) -> String {
    let mut parts = vec![app.label.clone(), "read-only".to_string()];
    if let Some(hint) = &app.hint {
        parts.push(hint.clone());
    }
    if let Some(page) = app.page() {
        let end = page.offset + page.rows.len() as u32;
        let start = if page.rows.is_empty() {
            page.offset
        } else {
            page.offset + 1
        };
        parts.push(format!("rows {start}-{end}"));
        if let Some(column) = page.columns.get(app.col_cursor()) {
            parts.push(format!("{} {}", column.name, column.type_name));
            if let Some(cell) = page
                .rows
                .get(app.row_cursor())
                .and_then(|row| row.get(app.col_cursor()))
                && !cell.is_empty()
            {
                parts.push(cell.clone());
            }
        }
    }
    if !app.status.is_empty() {
        parts.push(app.status.clone());
    }
    parts.join(" · ")
}
