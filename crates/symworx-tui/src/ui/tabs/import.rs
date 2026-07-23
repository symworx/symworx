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
        Paragraph,
    },
    Frame,
};

use crate::app::App;

pub fn render_import_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.help_mode {
        let help = Paragraph::new(
            "Import — BioSym file list & generate\n\
             Close help:  Esc  or  Alt-?\n\n\
             \n\
             FILES\n\n\
               ↑ ↓                 navigate list\n\
               Enter               load selected (or typed path)\n\
               /                   filter mode (type to narrow)\n\
               Esc / Enter         leave filter (filter text kept)\n\
               c                   convert selected → CSV\n\
               Ctrl+R  /  F5       refresh discovery\n\
               type…               manual path (Esc clears)\n\n\
             Multi-column CSVs open a column picker (number keys).\n\
             Discovered under ./data and the project root.\n\n\
             \n\
             GENERATE  (Ctrl+G)\n\n\
               1                   Resting PPG + ground-truth peaks\n\
               2                   Respiration + known peaks\n\
               3                   Stride intervals (event series)\n\
               Esc                 cancel generate menu\n\n\
             Peaks land in sidecar *.peaks.csv when applicable.\n\n\
             \n\
             NEXT\n\n\
               Ctrl+2              Explore (peaks, process, live)\n\
               Alt-? on Explore    full BioSym analysis help\n\n\
             \n\
             GLOBAL\n\n\
               Ctrl+H              Home\n\
               Esc Esc / Ctrl+Q    quit (Esc-Esc only at roots)\n",
        )
        .block(
            Block::new()
                .borders(Borders::ALL)
                .title(" Help — Import (BioSym) "),
        );
        frame.render_widget(help, area);
        return;
    }

    if let Some(pending) = &app.pending_load {
        let mut lines = vec![
            format!("\n\nFile: {}\n", pending.path.display()),
            format!("This file contains {} columns.\n\n", pending.columns),
            "Press the number key for the column you want to load as the main series:\n\n"
                .to_string(),
        ];

        if let Some(headers) = &pending.headers {
            for (i, name) in headers.iter().enumerate() {
                lines.push(format!("  {} = {} (column {})\n", i + 1, name, i));
            }
        } else {
            for i in 0..pending.columns {
                lines.push(format!("  {} = Column {}\n", i + 1, i));
            }
        }

        let content = Paragraph::new(lines.join(""))
            .block(Block::new().borders(Borders::ALL).title(" Column Picker "));
        frame.render_widget(content, area);
        return;
    }

    let show_header = app.filter_mode || !app.file_filter.is_empty() || !app.manual_path.is_empty();

    let (list_area, header_area) = if show_header {
        let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(3)]).split(area);
        (chunks[1], Some(chunks[0]))
    } else {
        (area, None)
    };

    if let Some(harea) = header_area {
        let mut h = String::new();
        if app.filter_mode || !app.file_filter.is_empty() {
            h.push_str(&format!("Filter: {}  ", app.file_filter));
        }
        if !app.manual_path.is_empty() {
            h.push_str(&format!("Manual: {}", app.manual_path));
        }
        let hp = Paragraph::new(h).style(Style::default().fg(Color::Yellow));
        frame.render_widget(hp, harea);
    }

    let vis = app.visible_indices();
    let title = if !app.file_filter.is_empty() {
        format!(
            " Import ({} / {} matching '{}') ",
            vis.len(),
            app.file_list.len(),
            app.file_filter
        )
    } else {
        " Import (file discovery) ".to_string()
    };

    let block = Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Color::Blue);

    let items: Vec<ListItem> = vis
        .iter()
        .map(|&orig| ListItem::new(app.file_list[orig].display().to_string()))
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, list_area, &mut app.list_state);
}
