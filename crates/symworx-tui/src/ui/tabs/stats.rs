// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! StatsSym — tabular statistics lab (Import · Lab · Generate).

use ratatui::{
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
    symbols,
    text::{
        Line,
        Span,
    },
    widgets::{
        Axis,
        Block,
        Borders,
        Chart,
        Dataset,
        GraphType,
        List,
        ListItem,
        Padding,
        Paragraph,
    },
    Frame,
};

use crate::app::{
    App,
    ResidualPanelMode,
    StatsLabResult,
    StatsLabTask,
    StatsView,
};

pub fn render_stats_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.help_mode {
        let body = match app.stats_view {
            StatsView::Import => {
                "StatsSym — Import (like BioSym Import)\n\
                 Close help:  Esc  or  Alt-?\n\n\
                 \n\
                 FILES\n\n\
                   ↑ ↓                 navigate discovered files\n\
                   Enter               load selected (or typed path) → Lab\n\
                   /                   filter mode\n\
                   x                   delete selected (y confirm / n Esc cancel)\n\
                   type…               manual path (Esc clears)\n\
                   Ctrl+R  /  F5       refresh discovery\n\n\
                 Numeric CSV with headers; non-numeric columns skipped.\n\n\
                 \n\
                 GENERATE\n\n\
                   Ctrl+G              open Generate tab (presets)\n\
                   Ctrl+← / Ctrl+→     Import · Lab · Generate\n\
                   Ctrl+1/2/3          jump Import / Lab / Generate\n\n\
                 \n\
                 GLOBAL\n\n\
                   Ctrl+H              Home\n\
                   Esc Esc / Ctrl+Q    quit (at Import root)\n"
            }
            StatsView::Lab => {
                "StatsSym — Lab (workspace, like BioSym Explore)\n\
                 Close help:  Esc  or  Alt-?\n\n\
                 \n\
                 After Import or Generate you land here with the table loaded.\n\
                 Column names show in the header and status (x/y cycles names).\n\n\
                 TASKS\n\n\
                   t / T               cycle task\n\
                   1–6                 Describe · Correlate · Fit OLS · Fit Poly\n\
                                       · Classify · Pipeline\n\
                   x / X               feature (X) column − / +\n\
                   y / Y               target (Y) column − / +\n\
                   Enter               run analysis\n\
                   h                   residual panel: Bland–Altman ↔ histogram\n\
                   Esc                 back to Import\n\
                   Ctrl+←→             module tabs\n\n\
                 FIT POLY\n\n\
                   Degree search 0..=max (crate polyreg)\n\
                   Left: degree table  R²  adjR²  AIC\n\
                   ★ = min AIC (preferred)  ·  ☆ = max R² if different\n\
                   Focus row note: nested χ² vs d−1 + p  ·  RMSE  ·  BIC  ·  β\n\
                   Right: fit + residuals for ▶ focused degree\n\
                   Under table: best-by-AIC summary\n\
                   ↑ ↓ / f             focus degree (plots follow)\n\
                   d / D               max degree +/−  (Enter re-run)\n\n\
                 CLASSIFY\n\n\
                   logistic binary (2 classes) or OVR multiclass (3+)\n\
                   y rounded to integer labels · X = all other cols\n\
                   plot: P(class) or confidence · confusion in summary\n\
                   demos: TwoClassBlobs · ThreeClassBlobs\n\n\
                 PIPELINE\n\n\
                   Left: splits table  ·  Right: plots for ★ row\n\
                   m / M               model OLS ↔ Logistic\n\
                   k / K               folds −/+  (Enter re-run)\n\
                   ↑ ↓ / f             focus split\n\
                   OLS: R² / RMSE / MAE · ŷ vs y\n\
                   Logistic: Acc / bal_acc / macro-F1 · true vs pred\n\
                   3-group story: ThreeClassBlobs → Classify → Pipeline+m Logistic\n"
            }
            StatsView::Generate => {
                "StatsSym — Generate synthetic data\n\
                 Close help:  Esc  or  Alt-?\n\n\
                 \n\
                 Open via Ctrl+G or Ctrl+→ from Lab / Ctrl+3.\n\n\
                 PRESETS  (symworx-stats::synthetic)\n\n\
                   ↑ ↓                 select preset\n\
                   n / N               sample size − / +\n\
                   s / S               seed − / +\n\
                   + / −               noise − / +\n\
                   Enter               generate CSV → load → jump to Lab\n\
                   Esc                 back to Import\n\n\
                 Linear regression → Lab task Regress; bivariate → Correlate;\n\
                 others → Describe. Press Enter in Lab to run.\n"
            }
        };
        frame.render_widget(
            Paragraph::new(body).block(
                Block::new()
                    .borders(Borders::ALL)
                    .padding(Padding::horizontal(1))
                    .title(" Help — StatsSym "),
            ),
            area,
        );
        return;
    }

    match app.stats_view {
        StatsView::Import => render_import(frame, app, area),
        StatsView::Lab => {
            let outer = Block::new()
                .title(" StatsSym — Lab ")
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .border_style(Color::Magenta);
            let inner = outer.inner(area);
            frame.render_widget(outer, area);
            render_lab(frame, app, inner);
        }
        StatsView::Generate => {
            let outer = Block::new()
                .title(" StatsSym — Generate ")
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .border_style(Color::Magenta);
            let inner = outer.inner(area);
            frame.render_widget(outer, area);
            render_generate(frame, app, inner);
        }
    }
}

fn render_lab(frame: &mut Frame, app: &App, area: Rect) {
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
        StatsLabTask::Poly => format!(
            "poly max d={}  ·  d/D  ·  ↑↓/f degree focus",
            app.stats_poly_max_degree
        ),
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
    let tab_cells = Layout::horizontal(tab_constraints)
        .spacing(1)
        .split(hdr_rows[0]);

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
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
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
            let body = Layout::horizontal([Constraint::Percentage(34), Constraint::Percentage(66)])
                .split(outer[2]);
            render_split_metrics_table(frame, body[0], r);
            let plots = Layout::vertical([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(body[1]);
            render_fit_panel(frame, plots[0], r);
            render_residual_panel(frame, plots[1], r, app.stats_residual_mode);
        } else {
            let plots = Layout::vertical([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(outer[2]);
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
        Paragraph::new(
            "1–6 task  x/y  Enter  h res  d poly  m model  k folds  ↑↓/f split  Esc Import",
        )
        .style(Style::default().fg(Color::DarkGray)),
        outer[3],
    );
}

fn render_split_metrics_table(frame: &mut Frame, area: Rect, r: &StatsLabResult) {
    use crate::app::SplitMetricKind;

    let is_clf = r
        .metrics_rows
        .first()
        .map(|row| row.metric_kind == SplitMetricKind::Classification)
        .unwrap_or(false);
    let is_poly =
        r.task == StatsLabTask::Poly || r.metrics_table_title.eq_ignore_ascii_case("degrees");
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
        if is_poly {
            " Degrees "
        } else {
            " Splits "
        }
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
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
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
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
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
fn return_static_title(s: &str) -> &'static str {
    // Prefer known titles without leak; fallback to Spaces-padded static-ish via match.
    match s {
        "Degrees" | "degrees" => " Degrees ",
        "Splits" | "splits" => " Splits ",
        _ => " Metrics ",
    }
}

fn render_fit_panel(frame: &mut Frame, area: Rect, r: &StatsLabResult) {
    let (sx, sy, flx, fly, _ba, _res, x_lab, y_lab, is_pred) = r.active_plot();
    if sx.is_empty() || sy.is_empty() {
        frame.render_widget(
            Paragraph::new("No scatter data").block(
                Block::new()
                    .borders(Borders::ALL)
                    .padding(Padding::horizontal(1))
                    .title(" Fit / observed "),
            ),
            area,
        );
        return;
    }

    let pts: Vec<(f64, f64)> = sx.iter().zip(sy.iter()).map(|(&x, &y)| (x, y)).collect();
    let line: Vec<(f64, f64)> = flx.iter().zip(fly.iter()).map(|(&x, &y)| (x, y)).collect();

    let x_min = pts
        .iter()
        .chain(line.iter())
        .map(|p| p.0)
        .fold(f64::INFINITY, f64::min);
    let mut x_max = pts
        .iter()
        .chain(line.iter())
        .map(|p| p.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let y_min = pts
        .iter()
        .chain(line.iter())
        .map(|p| p.1)
        .fold(f64::INFINITY, f64::min);
    let mut y_max = pts
        .iter()
        .chain(line.iter())
        .map(|p| p.1)
        .fold(f64::NEG_INFINITY, f64::max);
    if !x_min.is_finite() {
        return;
    }
    let x0 = if x_min >= 0.0 { 0.0 } else { x_min * 1.05 };
    let y0 = if y_min >= 0.0 { 0.0 } else { y_min * 1.05 };
    if (x_max - x0).abs() < 1e-12 {
        x_max = x0 + 1.0;
    }
    if (y_max - y0).abs() < 1e-12 {
        y_max = y0 + 1.0;
    }
    x_max *= 1.02;
    y_max = if y_max > 0.0 { y_max * 1.05 } else { y_max };

    let mut datasets = vec![Dataset::default()
        .name("observed")
        .marker(symbols::Marker::Dot)
        .graph_type(GraphType::Scatter)
        .style(Style::default().fg(Color::LightCyan))
        .data(&pts)];
    if line.len() >= 2 {
        datasets.push(
            Dataset::default()
                .name("fitted")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::LightYellow))
                .data(&line),
        );
    }

    let split_tag = r
        .metrics_rows
        .get(r.focused_row)
        .map(|row| format!(" · {}", row.label))
        .unwrap_or_default();
    let title = if is_pred {
        format!(" ŷ vs y{split_tag} · cyan=obs  yellow=identity ")
    } else {
        format!(" observed + fit{split_tag} ")
    };

    let chart = Chart::new(datasets)
        .block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(title)
                .border_style(Style::default().fg(Color::LightCyan)),
        )
        .x_axis(
            Axis::default()
                .title(x_lab)
                .bounds([x0, x_max])
                .labels(vec![
                    Line::from(format!("{x0:.1}")),
                    Line::from(format!("{:.1}", (x0 + x_max) / 2.0)),
                    Line::from(format!("{x_max:.1}")),
                ]),
        )
        .y_axis(
            Axis::default()
                .title(y_lab)
                .bounds([y0, y_max])
                .labels(vec![
                    Line::from(format!("{y0:.1}")),
                    Line::from(format!("{:.1}", (y0 + y_max) / 2.0)),
                    Line::from(format!("{y_max:.1}")),
                ]),
        );
    frame.render_widget(chart, area);
}

fn render_residual_panel(
    frame: &mut Frame,
    area: Rect,
    r: &StatsLabResult,
    mode: ResidualPanelMode,
) {
    let (_sx, _sy, _flx, _fly, ba_mean, residuals, _xl, _yl, _pred) = r.active_plot();
    if residuals.is_empty() {
        frame.render_widget(
            Paragraph::new("No residuals (run Regress or Pipeline).").block(
                Block::new()
                    .borders(Borders::ALL)
                    .padding(Padding::horizontal(1))
                    .title(" Residuals "),
            ),
            area,
        );
        return;
    }

    match mode {
        ResidualPanelMode::BlandAltman => {
            let pts: Vec<(f64, f64)> = ba_mean
                .iter()
                .zip(residuals.iter())
                .map(|(&m, &e)| (m, e))
                .collect();
            if pts.is_empty() {
                let pts: Vec<(f64, f64)> = residuals
                    .iter()
                    .enumerate()
                    .map(|(i, &e)| (i as f64, e))
                    .collect();
                render_ba_chart(frame, area, &pts, "index", "residual y−ŷ");
            } else {
                render_ba_chart(frame, area, &pts, "mean (y+ŷ)/2", "difference y−ŷ");
            }
        }
        ResidualPanelMode::Histogram => {
            render_residual_hist(frame, area, residuals);
        }
    }
}

fn render_ba_chart(frame: &mut Frame, area: Rect, pts: &[(f64, f64)], x_lab: &str, y_lab: &str) {
    let x_min = pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let x_max = pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
    let y_abs = pts
        .iter()
        .map(|p| p.1.abs())
        .fold(0.0_f64, f64::max)
        .max(1e-6);
    let y_lim = y_abs * 1.15;
    let x0 = if x_min >= 0.0 { 0.0 } else { x_min };
    let x1 = if (x_max - x0).abs() < 1e-12 {
        x0 + 1.0
    } else {
        x_max * 1.02
    };

    // mean residual ± 1.96 sd (BA limits)
    let n = pts.len() as f64;
    let mean_e = pts.iter().map(|p| p.1).sum::<f64>() / n;
    let var = pts.iter().map(|p| (p.1 - mean_e).powi(2)).sum::<f64>() / n.max(1.0);
    let sd = var.sqrt();
    let lo = mean_e - 1.96 * sd;
    let hi = mean_e + 1.96 * sd;
    let zero_line = vec![(x0, 0.0), (x1, 0.0)];
    let mean_line = vec![(x0, mean_e), (x1, mean_e)];
    let lo_line = vec![(x0, lo), (x1, lo)];
    let hi_line = vec![(x0, hi), (x1, hi)];

    let data: Vec<(f64, f64)> = pts.to_vec();
    let datasets = vec![
        Dataset::default()
            .name("points")
            .marker(symbols::Marker::Dot)
            .graph_type(GraphType::Scatter)
            .style(Style::default().fg(Color::LightMagenta))
            .data(&data),
        Dataset::default()
            .name("zero")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Gray))
            .data(&zero_line),
        Dataset::default()
            .name("mean")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Yellow))
            .data(&mean_line),
        Dataset::default()
            .name("loa")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::DarkGray))
            .data(&lo_line),
        Dataset::default()
            .name("loa_hi")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::DarkGray))
            .data(&hi_line),
    ];

    let chart = Chart::new(datasets)
        .block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(format!(
                    " Bland–Altman · mean e={mean_e:.3}  ±1.96 sd  h=hist "
                ))
                .border_style(Style::default().fg(Color::LightMagenta)),
        )
        .x_axis(Axis::default().title(x_lab).bounds([x0, x1]).labels(vec![
            Line::from(format!("{x0:.1}")),
            Line::from(format!("{:.1}", (x0 + x1) / 2.0)),
            Line::from(format!("{x1:.1}")),
        ]))
        .y_axis(
            Axis::default()
                .title(y_lab)
                .bounds([-y_lim, y_lim])
                .labels(vec![
                    Line::from(format!("{:.1}", -y_lim)),
                    Line::from("0"),
                    Line::from(format!("{y_lim:.1}")),
                ]),
        );
    frame.render_widget(chart, area);
}

fn render_residual_hist(frame: &mut Frame, area: Rect, residuals: &[f64]) {
    const NBINS: usize = 24;
    let mn = residuals.iter().copied().fold(f64::INFINITY, f64::min);
    let mx = residuals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = (mx - mn).max(1e-9);
    let mut counts = vec![0u64; NBINS];
    for &e in residuals {
        let t = ((e - mn) / span * (NBINS as f64 - 1e-9)).floor() as usize;
        counts[t.min(NBINS - 1)] += 1;
    }
    let max_c = counts.iter().copied().max().unwrap_or(1).max(1);
    // Fake scatter as bar tops for a simple histogram look
    let mut tops: Vec<(f64, f64)> = Vec::new();
    for (i, &c) in counts.iter().enumerate() {
        let x = mn + (i as f64 + 0.5) * span / NBINS as f64;
        tops.push((x, c as f64));
    }
    let y_max = max_c as f64 * 1.1;
    let data = tops;
    let chart = Chart::new(vec![Dataset::default()
        .name("hist")
        .marker(symbols::Marker::Block)
        .graph_type(GraphType::Scatter)
        .style(Style::default().fg(Color::LightYellow))
        .data(&data)])
    .block(
        Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .title(" Residual histogram  ·  h=Bland–Altman ")
            .border_style(Style::default().fg(Color::LightYellow)),
    )
    .x_axis(
        Axis::default()
            .title("residual y−ŷ")
            .bounds([mn, mx])
            .labels(vec![
                Line::from(format!("{mn:.2}")),
                Line::from("0"),
                Line::from(format!("{mx:.2}")),
            ]),
    )
    .y_axis(
        Axis::default()
            .title("count")
            .bounds([0.0, y_max])
            .labels(vec![
                Line::from("0"),
                Line::from(format!("{:.0}", y_max / 2.0)),
                Line::from(format!("{y_max:.0}")),
            ]),
    );
    frame.render_widget(chart, area);
}

fn render_generate(frame: &mut Frame, app: &App, area: Rect) {
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
        Line::from(Span::styled(
            p.description(),
            Style::default().fg(Color::Yellow),
        )),
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
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
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

/// Import: file discovery like BioSym, plus optional last-table column strip.
fn render_import(frame: &mut Frame, app: &mut App, area: Rect) {
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

fn render_placeholder(frame: &mut Frame, area: Rect, title: &str, body: &str) {
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
