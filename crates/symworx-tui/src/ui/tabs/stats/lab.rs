// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

use ratatui::{
    Frame,
    layout::{
        Alignment,
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

use super::{
    charts::{
        render_fit_panel,
        render_residual_panel,
    },
    placeholder::render_placeholder,
};
use crate::app::{
    App,
    ResidualPanelMode,
    StatsLabResult,
    StatsLabTask,
};

pub fn render_lab(frame: &mut Frame, app: &App, area: Rect) {
    let Some(ref table) = app.stats_table else {
        render_placeholder(
            frame,
            area,
            " 2. Lab ",
            "No table loaded.\n\n\
             Import a CSV or Generate synthetic data,\n\
             then press Enter to run.",
        );
        return;
    };

    let tasks = StatsLabTask::ALL;
    let xc = app.stats_lab_x_col.min(table.n_cols().saturating_sub(1));
    let yc = app.stats_lab_y_col.min(table.n_cols().saturating_sub(1));
    let xname = table.headers.get(xc).map(|s| s.as_str()).unwrap_or("?");
    let yname = table.headers.get(yc).map(|s| s.as_str()).unwrap_or("?");
    let outer = Layout::vertical([
        Constraint::Length(8), // tabs + Lab line + meta + mode (borders included)
        Constraint::Length(4),
        Constraint::Min(10),
        Constraint::Length(1),
    ])
    .split(area);

    let task = tasks[app.stats_lab_task.min(tasks.len() - 1)];
    let pipe_hint = match task {
        StatsLabTask::Pipeline => format!(
            "model={}  k={}  ·  m model  ↑↓/f split",
            app.stats_pipeline_model.label(),
            app.stats_pipeline_k
        ),
        StatsLabTask::Poly => format!("poly max d={}  ·  d/D  ·  ↑↓/f degree focus", app.stats_poly_max_degree),
        StatsLabTask::Classify => "logistic binary / OVR  ·  bal_acc + F1".into(),
        StatsLabTask::Regress => "single-X OLS  ·  fit + residuals".into(),
        StatsLabTask::Correlate => "Pearson r  ·  scatter".into(),
        StatsLabTask::Describe => "column summary".into(),
    };

    // Outer Lab chrome — no top title (label lives under the analysis strip).
    let lab_block = Block::new()
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1))
        .border_style(Color::Magenta);
    let lab_inner = lab_block.inner(outer[0]);
    frame.render_widget(lab_block, outer[0]);

    let hdr_rows = Layout::vertical([
        Constraint::Length(2), // analysis tabs flush to top of panel
        Constraint::Length(1), // "Lab" + spacer under selection
        Constraint::Length(1), // X/Y + size
        Constraint::Length(1), // mode hint + residual
    ])
    .split(lab_inner);

    // —— Analysis strip: equal-width cells, first row of the Lab panel ——
    let n_tasks = tasks.len().max(1);
    let tab_constraints: Vec<Constraint> = (0..n_tasks).map(|_| Constraint::Fill(1)).collect();
    let tab_cells = Layout::horizontal(tab_constraints).spacing(1).split(hdr_rows[0]);

    let sel = app.stats_lab_task.min(tasks.len() - 1);
    for (i, t) in tasks.iter().enumerate() {
        let selected = i == sel;
        let label = format!("{}.{}", i + 1, t.label());
        let style = if selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Magenta)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };
        let cell = Paragraph::new(vec![
            Line::from(Span::styled(format!(" {label} "), style)),
            Line::from(if selected {
                Span::styled(" ───── ", Style::default().fg(Color::Magenta))
            } else {
                Span::raw("")
            }),
        ])
        .alignment(Alignment::Center);
        frame.render_widget(cell, tab_cells[i]);
    }

    // Former top chrome line — now under the model selection.
    let under = Paragraph::new(Line::from(vec![
        Span::styled(
            " Lab ",
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " ·  statistical analyses  ·  1–6 or t to switch ",
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    frame.render_widget(under, hdr_rows[1]);

    let meta = Paragraph::new(format!(
        " X[{xc}]={xname}   Y[{yc}]={yname}   ·   {}×{}   ·   x/y cols   Enter run   h residual ",
        table.n_rows(),
        table.n_cols()
    ))
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(meta, hdr_rows[2]);

    let mode_line = Paragraph::new(format!(
        " {}   ·   residual: {} ",
        pipe_hint,
        match app.stats_residual_mode {
            ResidualPanelMode::BlandAltman => "Bland–Altman (h → hist)",
            ResidualPanelMode::Histogram => "histogram (h → BA)",
        }
    ))
    .style(Style::default().fg(Color::Yellow));
    frame.render_widget(mode_line, hdr_rows[3]);

    // Summary
    let (sum, interp) = if let Some(ref r) = app.stats_lab_result {
        (r.summary.clone(), r.interpretation.clone())
    } else {
        let tip = if !app.stats_gen_notes.is_empty() {
            app.stats_gen_notes.clone()
        } else {
            "Tip: Generate → Linear regression · Lab → Pipeline for split table.".into()
        };
        ("No run yet — press Enter.".into(), tip)
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(sum),
            Line::from(Span::styled(interp, Style::default().fg(Color::Yellow))),
        ])
        .block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(" Results "),
        ),
        outer[1],
    );

    // Body: pipeline = table | plots; else = stacked fit + residual
    if let Some(ref r) = app.stats_lab_result {
        if !r.metrics_rows.is_empty() {
            let body = Layout::horizontal([Constraint::Percentage(34), Constraint::Percentage(66)]).split(outer[2]);
            render_split_metrics_table(frame, body[0], r);
            let plots = Layout::vertical([Constraint::Percentage(55), Constraint::Percentage(45)]).split(body[1]);
            render_fit_panel(frame, plots[0], r);
            render_residual_panel(frame, plots[1], r, app.stats_residual_mode);
        } else {
            let plots = Layout::vertical([Constraint::Percentage(55), Constraint::Percentage(45)]).split(outer[2]);
            render_fit_panel(frame, plots[0], r);
            render_residual_panel(frame, plots[1], r, app.stats_residual_mode);
        }
    } else {
        frame.render_widget(
            Paragraph::new("Run an analysis to see observed + fitted (and residuals).").block(
                Block::new()
                    .borders(Borders::ALL)
                    .padding(Padding::horizontal(1))
                    .title(" Fit / observed "),
            ),
            outer[2],
        );
    }

    frame.render_widget(
        Paragraph::new("1–6 task  x/y  Enter  h res  d poly  m model  k folds  ↑↓/f split  Esc Import")
            .style(Style::default().fg(Color::DarkGray)),
        outer[3],
    );
}

pub fn render_split_metrics_table(frame: &mut Frame, area: Rect, r: &StatsLabResult) {
    use crate::app::SplitMetricKind;

    let is_clf = r
        .metrics_rows
        .first()
        .map(|row| row.metric_kind == SplitMetricKind::Classification)
        .unwrap_or(false);
    let is_poly = r.task == StatsLabTask::Poly || r.metrics_table_title.eq_ignore_ascii_case("degrees");
    let (h1, h2, h3) = if is_clf {
        ("Acc", "balAcc", "mF1")
    } else if is_poly {
        // Poly: R² + adj-R² + AIC (selection); χ²Δ lives in row notes / Best panel
        ("R²", "adjR²", "AIC")
    } else {
        ("R²", "RMSE", "MAE")
    };
    let col0 = if is_poly { "degree" } else { "split" };
    let title = if r.metrics_table_title.is_empty() {
        if is_poly { " Degrees " } else { " Splits " }
    } else {
        // pad title for block
        return_static_title(&r.metrics_table_title)
    };

    // Split panel: table rows + footer for best model under the table
    let chunks = Layout::vertical([
        Constraint::Min(6),
        Constraint::Length(if r.table_footer.is_empty() { 2 } else { 4 }),
    ])
    .split(area);

    let mut lines = vec![
        Line::from(Span::styled(
            format!(
                "  model: {}",
                if r.model_label.is_empty() {
                    "—"
                } else {
                    &r.model_label
                }
            ),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            format!("  {col0:<8}  n  {h1:>6} {h2:>7} {h3:>6}"),
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
        )),
    ];
    for (i, row) in r.metrics_rows.iter().enumerate() {
        let focus = i == r.focused_row;
        let style = if focus {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Magenta)
                .add_modifier(Modifier::BOLD)
        } else if row.is_best {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        // Focus marker (▶) separate from best (★ already in label for poly)
        let mark = if focus { "▶" } else { " " };
        lines.push(Line::from(Span::styled(
            format!(
                "{mark}{:<8} {:>4} {:>6.3} {:>7.3} {:>6.3}",
                row.label, row.n, row.r2, row.rmse, row.mae
            ),
            style,
        )));
    }
    lines.push(Line::from(Span::styled(
        if is_poly {
            "  ↑↓/f focus degree  ·  plots →"
        } else if is_clf {
            "  ↑↓/f focus  ·  true vs pred →"
        } else {
            "  ↑↓/f focus  ·  ŷ vs y →"
        },
        Style::default().fg(Color::DarkGray),
    )));

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(title)
                .border_style(Color::Magenta),
        ),
        chunks[0],
    );

    // Under table: best-fit / hold-out blurb
    let footer_text = if !r.table_footer.is_empty() {
        r.table_footer.clone()
    } else if let Some(bi) = r.best_row {
        if let Some(row) = r.metrics_rows.get(bi) {
            format!(
                "★ best: {}  R²={:.4}  RMSE={:.4}\n  {}",
                row.label, row.r2, row.rmse, row.note
            )
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    frame.render_widget(
        Paragraph::new(footer_text).block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(if is_poly { " Best model " } else { " Summary " })
                .border_style(Color::Yellow),
        ),
        chunks[1],
    );
}

/// Block title with spaces from a dynamic string (leaked for 'static title API).
pub fn return_static_title(s: &str) -> &'static str {
    // Prefer known titles without leak; fallback to Spaces-padded static-ish via match.
    match s {
        "Degrees" | "degrees" => " Degrees ",
        "Splits" | "splits" => " Splits ",
        _ => " Metrics ",
    }
}
