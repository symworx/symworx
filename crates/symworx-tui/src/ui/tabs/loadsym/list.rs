use ratatui::{
    Frame,
    layout::Rect,
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

use super::util::truncate_str;
use crate::app::App;

pub fn render_loadsym_list(frame: &mut Frame, app: &App, area: Rect) {
    let sel = app.loadsym_selection;

    let lines = vec![
        Line::from(Span::styled(
            "Select view (↑↓ or 1–4, Enter):",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
        Line::from(format!("{}1. Workout Analysis", if sel == 0 { "▶ " } else { "  " })),
        Line::from(Span::styled(
            "   Charts: power/HR/speed/cad/elev · 1–5 toggle · o open · SEPi/TSLi",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(format!("{}2. Metrics / Library", if sel == 1 { "▶ " } else { "  " })),
        Line::from(Span::styled(
            "   Table + trend / bi-plot · 1–8 metrics · Enter open workout",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(format!("{}3. Calendar View", if sel == 2 { "▶ " } else { "  " })),
        Line::from(Span::styled(
            if app.loadsym_from_catalog {
                format!(
                    "   {} days from catalog  • multi-day TSLi + ACLi",
                    app.daily_loads.len()
                )
            } else {
                "   Multi-day load + ACLi (r: reload catalog, g: demo)".to_string()
            },
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(format!(
            "{}4. Programming Optimization",
            if sel == 3 { "▶ " } else { "  " }
        )),
        Line::from(Span::styled(
            "   Default goal from form/fatigue/ACLi · 1/2/3 override · chronic load bands",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "o: open file   i: newest   r: catalog   g: demo   Esc/Ctrl+H: back",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let p = Paragraph::new(lines).block(
        Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .title(" LoadSym — Home "),
    );
    frame.render_widget(p, area);
}

pub fn render_workout_open_modal(frame: &mut Frame, app: &App, area: Rect) {
    let n = app.workout_file_list.len();
    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            format!("Select activity file  ({n} found)  ·  ↑↓  Enter load  Esc cancel"),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    if n == 0 {
        lines.push(Line::from(Span::styled(
            "No .fit/.csv in $VELOFIT_HOME/raw|inbox or ./data|rides.",
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(Span::styled(
            "Drop a file or run: symload email fetch / ingest",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        let view_h = area.height.saturating_sub(6) as usize;
        let view_h = view_h.max(5);
        let sel = app.workout_file_sel.min(n - 1);
        let start = sel.saturating_sub(view_h / 3);
        let end = (start + view_h).min(n);
        for i in start..end {
            let path = &app.workout_file_list[i];
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("?");
            let parent = path.parent().map(|p| p.display().to_string()).unwrap_or_default();
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
                format!("{marker} {:<36}  {}", truncate_str(name, 36), truncate_str(&parent, 40)),
                style,
            )));
        }
    }
    let p = Paragraph::new(lines).block(
        Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .title(" Open workout file ")
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(p, area);
}
