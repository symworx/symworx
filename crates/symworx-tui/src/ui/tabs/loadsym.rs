use ratatui::{
    layout::{
        Constraint,
        Direction,
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
        Cell,
        Chart,
        Dataset,
        GraphType,
        Padding,
        Paragraph,
        Row,
        Sparkline,
        Table,
    },
    Frame,
};
use symworx_loadsym::load::{
    classify_acwr,
    compute_acute_chronic,
    compute_monotony,
    compute_ride_metrics,
    compute_strain,
    find_exceedance_regions,
    highest_rolling,
    simulate_pulse_response,
    LoadGoal,
    PulseResponseParams,
    MAX_HORIZON_DAYS,
};

use crate::app::{
    ActivityMetricsUiRow,
    App,
    LoadSymView,
    MetricsChartMode,
    MetricsField,
    WorkoutStream,
};

/// Render the LoadSym tab (now a proper selector + sub-views per plan).
pub fn render_loadsym_tab(frame: &mut Frame, app: &App, area: Rect) {
    if app.help_mode {
        let body = match app.loadsym_view {
            LoadSymView::List => {
                "LoadSym — home\n\
                 Close help:  Esc  or  Alt-?\n\n\
                 \n\
                 VIEWS\n\n\
                   ↑ ↓  or  1–4           select\n\
                   Enter                  open selected view\n\n\
                   1  Workout Analysis    single ride · charts · SEPi/TSLi\n\
                   2  Metrics / Library   per-ride table · trends · bi-plots\n\
                   3  Calendar            daily/weekly load · catalog\n\
                   4  Optimization        multi-day plan · form/fatigue\n\n\
                 \n\
                 SHORTCUTS ON THIS LIST\n\n\
                   o                      open activity file browser\n\
                   i                      load newest .fit/CSV\n\
                   r                      reload SQLite catalog\n\
                   g                      synthetic demo daily loads\n\n\
                 Archive: $VELOFIT_HOME (default ~/velofit).\n\
                 Catalog is personal (never in git).\n"
            }
            LoadSymView::Workout => {
                "LoadSym — Workout Analysis\n\
                 Close help:  Esc  or  Alt-?\n\n\
                 \n\
                 OPEN A RIDE\n\n\
                   o                      file browser (↑↓ Enter)\n\
                   i / a                  newest file under archive dirs\n\
                   From Calendar          n/p ride · Enter/o open here\n\
                   r                      clear loaded activity\n\n\
                 Panel layout is kept when reloading (i/o) until you clear.\n\n\
                 \n\
                 CHARTS  (line, BioSym-style)\n\n\
                   1  power (W)           toggle show/hide\n\
                   2  heart rate          remaining panels share height\n\
                   3  speed (km/h)\n\
                   4  cadence (rpm)\n\
                   5  elevation (m)\n\
                   ← →                    pan shared time window\n\
                   ● open  ○ closed  ∅ no data in file\n\n\
                 \n\
                 METRICS\n\n\
                   f / F                  FTP ±5 W (SEPi / TSLi)\n\
                   t / T                  threshold ±\n\
                   d / D                  min duration ±\n\
                   Esc                    back to LoadSym list\n"
            }
            LoadSymView::Calendar => {
                "LoadSym — Calendar\n\
                 Close help:  Esc  or  Alt-?\n\n\
                 \n\
                 NAVIGATION\n\n\
                   ↑ ↓  /  k j            day (newest first on screen)\n\
                   ← →  /  h l            week aggregate\n\
                   Home / End             first / last day\n\
                   PgUp / PgDn            jump 10 days\n\
                   .                      jump to most recent day\n\n\
                 \n\
                 RIDES ON FOCUSED DAY\n\n\
                   n / p                  next / previous file\n\
                   Enter  /  o            open in Workout Analysis\n\n\
                 \n\
                 DATA\n\n\
                   r                      reload catalog ($VELOFIT_HOME/db)\n\
                   g                      demo daily series\n\
                   Esc                    back to LoadSym list\n\n\
                 Metrics: TSLi, ACLi, monotony, strain (LOADsym names).\n"
            }
            LoadSymView::Optimization => {
                "LoadSym — Programming Optimization\n\
                 Close help:  Esc  or  Alt-?\n\n\
                 \n\
                 GOAL  (default from form / fatigue / ACLi)\n\n\
                   auto                   scored on enter / catalog reload\n\
                   1  Recovery            ~20–55% of chronic load\n\
                   2  Maintenance         ~85–115%C, modulated days\n\
                   3  Overload            ~115–140%C\n\
                   1/2/3                  override; sticks until re-enter\n\n\
                 \n\
                 PLAN\n\n\
                   − / +                  horizon days (2…10)\n\
                   Enter                  recompute plan\n\
                   r                      reload catalog (+ re-suggest if no override)\n\
                   g                      28-day demo loads + replan\n\
                   Esc                    back to LoadSym list\n\n\
                 Charts: recent load + readiness (history | projection).\n\
                 Success = chronic load band; ACLi is advisory only.\n"
            }
            LoadSymView::Metrics => {
                "LoadSym — Metrics / Library\n\
                 Close help:  Esc  or  Alt-?\n\n\
                 \n\
                 TABLE\n\n\
                   ↑ ↓                  select ride (newest first)\n\
                   PgUp / PgDn          jump 10\n\
                   Home / End           first / last\n\
                   Enter / o            open in Workout Analysis\n\
                   r                    reload catalog\n\n\
                 \n\
                 CHARTS  (below table)\n\n\
                   v                    toggle trend ↔ bi-plot\n\n\
                 Trend  (metric vs ride order):\n\
                   1–8                  pick Y field\n\
                     1 TSLi  2 SEPi  3 avgW  4 dur  5 avgHR\n\
                     6 SRIi  7 work  8 maxW\n\n\
                 Bi-plot  (X vs Y):\n\
                   x / X                cycle X axis\n\
                   y / Y                cycle Y axis\n\
                   1–8                  set Y quickly (same map)\n\n\
                 Focused table row is highlighted on the chart.\n\
                 Esc                  back to LoadSym list\n"
            }
        };
        let global = "\n\
             GLOBAL\n\n\
               Ctrl+H              Home\n\
               Esc Esc / Ctrl+Q    quit (Esc-Esc at roots only)\n\
               Alt-?               help\n";
        let help = Paragraph::new(format!("{body}{global}")).block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(match app.loadsym_view {
                    LoadSymView::List => " Help — LoadSym ",
                    LoadSymView::Workout => " Help — LoadSym · Workout ",
                    LoadSymView::Calendar => " Help — LoadSym · Calendar ",
                    LoadSymView::Optimization => " Help — LoadSym · Optimization ",
                    LoadSymView::Metrics => " Help — LoadSym · Metrics ",
                }),
        );
        frame.render_widget(help, area);
        return;
    }

    let outer = Block::new()
        .title(" LoadSym — Training Load, ACLi, Optimization ")
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1))
        .border_style(Color::Yellow);

    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    // File-open modal overlays Workout (and List→open) content.
    if app.pending_workout_open {
        render_workout_open_modal(frame, app, inner);
        return;
    }

    match app.loadsym_view {
        LoadSymView::List => {
            render_loadsym_list(frame, app, inner);
        }
        LoadSymView::Workout => {
            render_workout_view(frame, app, inner);
        }
        LoadSymView::Calendar => {
            render_calendar_view(frame, app, inner);
        }
        LoadSymView::Optimization => {
            render_optimization_view(frame, app, inner);
        }
        LoadSymView::Metrics => {
            render_metrics_view(frame, app, inner);
        }
    }
}

fn render_loadsym_list(frame: &mut Frame, app: &App, area: Rect) {
    let sel = app.loadsym_selection;

    let lines = vec![
        Line::from(Span::styled(
            "Select view (↑↓ or 1–4, Enter):",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
        Line::from(format!(
            "{}1. Workout Analysis",
            if sel == 0 { "▶ " } else { "  " }
        )),
        Line::from(Span::styled(
            "   Charts: power/HR/speed/cad/elev · 1–5 toggle · o open · SEPi/TSLi",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(format!(
            "{}2. Metrics / Library",
            if sel == 1 { "▶ " } else { "  " }
        )),
        Line::from(Span::styled(
            "   Table + trend / bi-plot · 1–8 metrics · Enter open workout",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(format!(
            "{}3. Calendar View",
            if sel == 2 { "▶ " } else { "  " }
        )),
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

fn render_workout_open_modal(frame: &mut Frame, app: &App, area: Rect) {
    let n = app.workout_file_list.len();
    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            format!("Select activity file  ({n} found)  ·  ↑↓  Enter load  Esc cancel"),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
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
            let parent = path
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
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
                format!(
                    "{marker} {:<36}  {}",
                    truncate_str(name, 36),
                    truncate_str(&parent, 40)
                ),
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

fn stream_color(s: WorkoutStream) -> Color {
    match s {
        WorkoutStream::Power => Color::LightGreen,
        WorkoutStream::HeartRate => Color::LightRed,
        WorkoutStream::Speed => Color::LightCyan,
        WorkoutStream::Cadence => Color::Yellow,
        WorkoutStream::Elevation => Color::White,
    }
}

fn stream_series(
    act: &symworx_io::ActivityData,
    s: WorkoutStream,
) -> (Vec<f64>, &'static str, bool) {
    (s.series(act), s.chart_title(), s.present_on(act))
}

/// Visible sample window for workout line charts (shared pan across panels).
const WORKOUT_VIEW_LEN: usize = 240;

fn render_workout_view(frame: &mut Frame, app: &App, area: Rect) {
    let Some(act) = app.loaded_activity.as_ref() else {
        let empty = Paragraph::new(vec![
            Line::from(Span::styled(
                "Workout Analysis — no activity loaded",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  o     open file browser ($VELOFIT_HOME/raw|inbox, ./data)"),
            Line::from("  i / a load newest activity file"),
            Line::from("  From Calendar: select day → n/p ride → Enter/o"),
            Line::from(""),
            Line::from(Span::styled(
                "After load: line charts for available streams (1–5 toggle panels).",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "  1 power  2 HR  3 speed  4 cadence  5 elevation",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(" 1. Workout Analysis "),
        );
        frame.render_widget(empty, area);
        return;
    };

    let label = format!("{} ({} pts)", act.source, act.times_s.len());
    let n = act.len().max(1);
    let view_len = WORKOUT_VIEW_LEN;
    let max_start = n.saturating_sub(view_len);
    let start = app.activity_scroll.min(max_start);
    let end = (start + view_len).min(n);

    // Focused series for summary / thresh (activity_series)
    let focus = WorkoutStream::from_index(app.activity_series.min(WorkoutStream::COUNT - 1))
        .unwrap_or(WorkoutStream::Power);
    let (focus_series, series_name, series_present) = stream_series(act, focus);

    let n_samples = focus_series.len().max(1);
    let best3 = if focus_series.is_empty() {
        0.0
    } else {
        highest_rolling(&focus_series, 3)
    };
    let best30 = if focus_series.is_empty() {
        0.0
    } else {
        highest_rolling(&focus_series, 30.min(focus_series.len()))
    };
    let mean: f64 = if focus_series.is_empty() {
        0.0
    } else {
        focus_series.iter().sum::<f64>() / n_samples as f64
    };
    let var = if focus_series.is_empty() {
        0.0
    } else {
        focus_series.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n_samples as f64
    };
    let std = var.sqrt();
    let peak = focus_series.iter().copied().fold(0.0_f64, f64::max);
    let use_user = app.workout_user_thresh > 0.0;
    let thresh = if use_user {
        app.workout_user_thresh
    } else {
        mean + 0.5 * std.max(1.0)
    };
    let min_dur = if use_user {
        app.workout_user_min_dur
    } else {
        1
    };
    let regions = if focus_series.is_empty() {
        0
    } else {
        find_exceedance_regions(&focus_series, thresh, min_dur).len()
    };

    let ride_metrics = if act.has_power() {
        let p = act.power_series();
        Some(compute_ride_metrics(&act.times_s, &p, app.ftp))
    } else {
        None
    };

    let panel_hint = WorkoutStream::ALL
        .iter()
        .map(|s| {
            let present = s.present_on(act);
            let on = app.workout_stream_on[s.index()];
            format!(
                "{} {}{}{}",
                s.key_digit(),
                s.short_label(),
                if present { "" } else { "∅" },
                if on { "●" } else { "○" },
            )
        })
        .collect::<Vec<_>>()
        .join(" · ");

    let header_row: Vec<Cell> = vec![
        "Mean", "Std", "Peak", "Best3", "Best30", "Regions", "Thresh", "SEPi", "TSLi",
    ]
    .into_iter()
    .map(|h| {
        Cell::from(h).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    })
    .collect();

    let (np_str, tss_str) = if let Some(ref m) = ride_metrics {
        (format!("{:.0}", m.np), format!("{:.1}", m.tss))
    } else {
        ("-".into(), "-".into())
    };

    let value_row: Vec<Cell> = vec![
        format!("{:.1}", mean),
        format!("{:.1}", std),
        format!("{:.0}", peak),
        format!("{:.0}", best3),
        format!("{:.0}", best30),
        format!("{}", regions),
        format!("{:.1}{}", thresh, if use_user { "*" } else { "" }),
        np_str,
        tss_str,
    ]
    .into_iter()
    .map(Cell::from)
    .collect();

    let summary_table = Table::new(
        vec![Row::new(header_row), Row::new(value_row)],
        vec![Constraint::Ratio(1, 9); 9],
    )
    .block(
        Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .title(format!(
                "Workout — {}  focus={}{}  FTP={:.0}W  pan {}..{}/{}",
                label,
                series_name,
                if series_present { "" } else { "∅" },
                app.ftp,
                start,
                end,
                n
            )),
    );

    let mut info_bits = Vec::new();
    if let Some(m) = &act.manufacturer {
        info_bits.push(format!("{} {}", m, act.product.as_deref().unwrap_or("")));
    }
    if let Some(sp) = &act.sport {
        info_bits.push(sp.clone());
    }
    info_bits.push(format!("{:.0}s", act.duration_s()));
    if let Some(ref m) = ride_metrics {
        info_bits.push(format!(
            "SEPi={:.0} SRIi={:.2} TSLi={:.1} {:.0}kJ",
            m.np, m.if_, m.tss, m.total_work_kj
        ));
    }
    let info = Paragraph::new(format!(
        "{}  |  {}  |  ←→ pan  1–5 toggle",
        info_bits.join(" · "),
        panel_hint
    ))
    .block(
        Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .title(" Ride "),
    );

    // Visible panels in stream order (closed streams omitted → equal height share).
    let mut panels: Vec<(WorkoutStream, Vec<f64>, bool)> = Vec::new();
    for s in WorkoutStream::ALL {
        if app.workout_stream_on[s.index()] {
            let (series, _, present) = stream_series(act, s);
            panels.push((s, series, present));
        }
    }
    if panels.is_empty() {
        let (series, _, present) = stream_series(act, WorkoutStream::Power);
        panels.push((WorkoutStream::Power, series, present));
    }

    let n_panels = panels.len();
    let mut constraints = vec![
        Constraint::Length(4), // summary
        Constraint::Length(3), // ride info
    ];
    for _ in 0..n_panels {
        constraints.push(Constraint::Min(5)); // equal-share body
    }
    constraints.push(Constraint::Length(1)); // footer
    let chunks = Layout::vertical(constraints).split(area);

    frame.render_widget(summary_table, chunks[0]);
    frame.render_widget(info, chunks[1]);

    for (i, (stream, series, present)) in panels.iter().enumerate() {
        let rect = chunks[2 + i];
        let title = if *present {
            format!(" {} ", stream.chart_title())
        } else {
            format!(" {} · no data ", stream.chart_title())
        };
        render_workout_line_chart(
            frame,
            rect,
            series,
            start,
            end,
            &title,
            stream_color(*stream),
            *present,
        );
    }

    let footer = Paragraph::new(
        "o open  i newest  r clear  ←→ pan  1–5 streams  t/T thresh  d/D dur  f/F FTP  Esc list",
    )
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[2 + n_panels]);
}

/// BioSym-style line chart for one workout channel (viewport [start, end)).
fn render_workout_line_chart(
    frame: &mut Frame,
    area: Rect,
    series: &[f64],
    start: usize,
    end: usize,
    title: &str,
    color: Color,
    present: bool,
) {
    let n = series.len();
    let end = end.min(n);
    let start = start.min(end);
    let visible: Vec<f64> = if start < end {
        series[start..end].to_vec()
    } else {
        vec![]
    };

    let mut y_min = visible.iter().copied().fold(f64::INFINITY, f64::min);
    let mut y_max = visible.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !y_min.is_finite() || !y_max.is_finite() || (y_max - y_min).abs() < 1e-12 {
        y_min = 0.0;
        y_max = 1.0;
    }
    let pad = (y_max - y_min) * 0.08;
    y_min -= pad;
    y_max += pad;
    if y_min < 0.0 && series.iter().all(|&v| v >= 0.0) {
        y_min = 0.0;
    }

    let data: Vec<(f64, f64)> = visible
        .iter()
        .enumerate()
        .map(|(i, &v)| ((start + i) as f64, v))
        .collect();

    let x_lo = start as f64;
    let x_hi = if end > start {
        (end - 1) as f64
    } else {
        start as f64 + 1.0
    };

    let style = if present {
        Style::default().fg(color)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let datasets = vec![Dataset::default()
        .name(title.trim())
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(style)
        .data(&data)];

    let chart = Chart::new(datasets)
        .block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(title.to_string())
                .border_style(style),
        )
        .x_axis(
            Axis::default()
                .style(Style::default().fg(Color::Gray))
                .bounds([x_lo, x_hi.max(x_lo + 1.0)])
                .labels(vec![
                    Line::from(format!("{start}")),
                    Line::from(format!("{}", (start + end) / 2)),
                    Line::from(format!("{end}")),
                ]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(Color::Gray))
                .bounds([y_min, y_max])
                .labels(vec![
                    Line::from(format!("{y_min:.0}")),
                    Line::from(format!("{:.0}", (y_min + y_max) / 2.0)),
                    Line::from(format!("{y_max:.0}")),
                ]),
        );

    frame.render_widget(chart, area);
}

fn render_calendar_view(frame: &mut Frame, app: &App, area: Rect) {
    let loads = &app.daily_loads;
    if loads.is_empty() {
        let empty = Paragraph::new(
            "No daily loads loaded.\n\n\
             • Press r to reload $VELOFIT_HOME/db/loadsym.sqlite\n\
             • Or g for synthetic demo series\n\
             • Or run: symload reprocess  (then r here)\n\n\
             Tip: Zwift/other undated files use FIT timestamps (not file mtime).",
        )
        .block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(" 2. Calendar View — empty "),
        );
        frame.render_widget(empty, area);
        return;
    }

    let day_i = app.loadsym_scroll.min(loads.len().saturating_sub(1));
    let week_i = if app.weekly_loads.is_empty() {
        0
    } else {
        app.loadsym_week_scroll
            .min(app.weekly_loads.len().saturating_sub(1))
    };

    let acwr_snap = compute_acute_chronic(loads, 7, 28).ok();
    let mono = compute_monotony(loads).unwrap_or(1.0);
    let strain = compute_strain(loads).unwrap_or(0.0);
    let latest_acwr = app
        .daily_acwr
        .get(day_i)
        .and_then(|a| *a)
        .or_else(|| acwr_snap.as_ref().map(|s| s.acwr))
        .unwrap_or(0.0);
    let risk_str = app
        .daily_risk
        .get(day_i)
        .and_then(|r| r.clone())
        .unwrap_or_else(|| {
            acwr_snap
                .as_ref()
                .map(|s| classify_acwr(s.acwr).as_str().to_string())
                .unwrap_or_else(|| "N/A".to_string())
        });
    let focus_date = app
        .daily_load_dates
        .get(day_i)
        .cloned()
        .unwrap_or_else(|| format!("day {}", day_i));
    let n_rides_day = app.daily_ride_counts.get(day_i).copied().unwrap_or(0);

    // Header → dual lists → weekly aggregate bar at bottom
    let outer = Layout::vertical([
        Constraint::Length(5), // header
        Constraint::Min(6),    // dual lists
        Constraint::Length(7), // weekly TSLi: * above + bars + * below + label
    ])
    .split(area);

    let source = if app.loadsym_from_catalog {
        "catalog"
    } else {
        "demo"
    };
    let week_focus = app.weekly_loads.get(week_i);
    let week_tss = week_focus.map(|w| w.total_tss).unwrap_or(0.0);
    let week_rides = week_focus.map(|w| w.ride_count).unwrap_or(0);
    let week_label = week_focus
        .map(|w| w.week_start.as_str())
        .unwrap_or("----------");
    let week_num = if app.weekly_loads.is_empty() {
        0
    } else {
        week_i + 1
    };
    let week_total = app.weekly_loads.len();

    // Fixed-width columns so values don't jump while scrolling
    //   DATE       TSLi     n │ WEEK       W-TSLi   rides │ ACLi  risk       mono  strain
    let col_hdr = format!(
        "{:<10}  {:>7}  {:>3}  │  {:<10}  {:>7}  {:>5}  │  {:>5}  {:<9}  {:>5}  {:>7}",
        "DATE", "TSLi", "n", "WEEK", "W-TSLi", "rides", "ACLi", "risk", "mono", "strain"
    );
    let col_vals = format!(
        "{:<10}  {:>7.1}  {:>3}  │  {:<10}  {:>7.1}  {:>5}  │  {:>5.2}  {:<9}  {:>5.2}  {:>7.1}",
        truncate_str(&focus_date, 10),
        loads[day_i],
        n_rides_day.min(999),
        truncate_str(week_label, 10),
        week_tss,
        week_rides.min(99999),
        latest_acwr,
        truncate_str(&risk_str, 9),
        mono,
        strain
    );

    let header = Paragraph::new(vec![
        Line::from(format!(
            "[{:<7}]  day {:>4}/{:<4}  week {:>4}/{:<4}   ↑↓ day  ←→ week  . latest  r reload",
            truncate_str(source, 7),
            day_i + 1,
            loads.len().min(9999),
            week_num,
            week_total.min(9999),
        )),
        Line::from(Span::styled(col_hdr, Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled(
            col_vals,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
    ])
    .block(
        Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .title(" 2. Calendar "),
    );
    frame.render_widget(header, outer[0]);

    // Dual columns: daily (left) + weekly (right), both fill remaining height
    let cols = Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(outer[1]);

    // --- Daily: list of days; for focus day, also list ride files ---
    let day_list_h = cols[0].height.saturating_sub(2) as usize; // borders
    let ride_lines_budget = (day_list_h / 3).clamp(3, 12);
    let day_rows_budget = day_list_h.saturating_sub(ride_lines_budget + 2).max(4);

    // Newest-first window: focus day near the top of the daily list
    let day_hi = day_i; // inclusive (most recent in window when at end)
    let day_lo = day_hi.saturating_sub(day_rows_budget.saturating_sub(1));

    let mut daily_lines: Vec<Line> = Vec::new();
    daily_lines.push(Line::from(Span::styled(
        "DAILY  (↑↓)  newest first",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    // Render newest → older so most recent is at the top of the pane
    daily_lines.push(Line::from(Span::styled(
        format!("  {:<10}  {:>7}  {:>3}  {:>6}", "date", "TSLi", "n", "ACLi"),
        Style::default().fg(Color::DarkGray),
    )));
    for idx in (day_lo..=day_hi).rev() {
        let marker = if idx == day_i { "▶" } else { " " };
        let label = app
            .daily_load_dates
            .get(idx)
            .cloned()
            .unwrap_or_else(|| format!("d{idx}"));
        let ac = app.daily_acwr.get(idx).and_then(|a| *a).unwrap_or(0.0);
        let nr = app.daily_ride_counts.get(idx).copied().unwrap_or(0);
        daily_lines.push(Line::from(format!(
            "{marker} {:<10}  {:>7.1}  {:>3}  {:>6.2}",
            truncate_str(&label, 10),
            loads[idx],
            nr.min(999),
            ac
        )));
    }

    daily_lines.push(Line::from(Span::styled(
        format!("— rides on {focus_date} —"),
        Style::default().fg(Color::DarkGray),
    )));
    let day_rides: Vec<_> = app
        .catalog_rides
        .iter()
        .filter(|r| r.ride_date == focus_date)
        .collect();
    if day_rides.is_empty() {
        daily_lines.push(Line::from(Span::styled(
            "  (no per-file rows — demo or empty day)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        let ride_sel = app.calendar_ride_sel.min(day_rides.len().saturating_sub(1));
        for (k, r) in day_rides
            .iter()
            .take(ride_lines_budget.saturating_sub(1))
            .enumerate()
        {
            let name = r.source_file.rsplit('/').next().unwrap_or(&r.source_file);
            let mins = r.duration_s / 60.0;
            let marker = if k == ride_sel { "▶" } else { " " };
            let style = if k == ride_sel {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            daily_lines.push(Line::from(Span::styled(
                format!(
                    "{marker}{:>2}. {:<24}  {:>7.1}  {:>5.0}m",
                    k + 1,
                    truncate_str(name, 24),
                    r.tss,
                    mins
                ),
                style,
            )));
        }
        if day_rides.len() > ride_lines_budget.saturating_sub(1) {
            daily_lines.push(Line::from(format!(
                "  … +{} more rides  (n/p cycle · Enter open)",
                day_rides.len() + 1 - ride_lines_budget
            )));
        } else {
            daily_lines.push(Line::from(Span::styled(
                "  n/p: cycle ride · Enter/o: open in Workout",
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    let daily_p = Paragraph::new(daily_lines).block(
        Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .title(" Daily ")
            .border_style(if !app.loadsym_scroll_from_week {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            }),
    );
    frame.render_widget(daily_p, cols[0]);

    // --- Weekly aggregates (newest first) ---
    let week_h = cols[1].height.saturating_sub(2) as usize;
    let week_rows = week_h.saturating_sub(1).max(1); // leave row for title
    let week_hi = week_i;
    let week_lo = week_hi.saturating_sub(week_rows.saturating_sub(1));

    let mut week_lines: Vec<Line> = Vec::new();
    week_lines.push(Line::from(Span::styled(
        "WEEKLY  (←→)  newest first",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    if app.weekly_loads.is_empty() {
        week_lines.push(Line::from("  (no weeks)"));
    } else {
        week_lines.push(Line::from(Span::styled(
            format!(
                "  {:<10}  {:>7}  {:>5}  {:>4}",
                "week", "W-TSLi", "rides", "days"
            ),
            Style::default().fg(Color::DarkGray),
        )));
        for wi in (week_lo..=week_hi).rev() {
            let w = &app.weekly_loads[wi];
            let marker = if wi == week_i { "▶" } else { " " };
            week_lines.push(Line::from(format!(
                "{marker} {:<10}  {:>7.1}  {:>5}  {:>4}",
                truncate_str(&w.week_start, 10),
                w.total_tss,
                w.ride_count.min(99999),
                w.day_count.min(9999)
            )));
        }
    }

    let week_p = Paragraph::new(week_lines).block(
        Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .title(" Weekly ")
            .border_style(if app.loadsym_scroll_from_week {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            }),
    );
    frame.render_widget(week_p, cols[1]);

    // --- Bottom bar: weekly TSLi aggregates + * under focused week ---
    render_weekly_tsli_bar(frame, app, outer[2], week_i);
}

/// Weekly TSLi sparkline across the bottom; `*` marks the focused week (above + below).
fn render_weekly_tsli_bar(frame: &mut Frame, app: &App, area: Rect, week_i: usize) {
    let n_weeks = app.weekly_loads.len();
    let title = if n_weeks == 0 {
        " weekly TSLi — no weeks loaded (r to reload catalog) ".to_string()
    } else if let Some(w) = app.weekly_loads.get(week_i) {
        format!(
            " weekly TSLi  ·  {} weeks  ·  focus {}  TSLi={:.0}  rides={}  * ",
            n_weeks, w.week_start, w.total_tss, w.ride_count
        )
    } else {
        format!(" weekly TSLi  ·  {} weeks ", n_weeks)
    };
    let block = Block::new()
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1))
        .border_style(Style::default().fg(Color::Yellow))
        .title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.weekly_loads.is_empty() || inner.height == 0 {
        return;
    }

    // marker above · bars · marker below  (taller bar region)
    let bar_rows = inner.height.saturating_sub(2).max(1);
    let split = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(bar_rows),
        Constraint::Length(1),
    ])
    .split(inner);

    let max_w = app
        .weekly_loads
        .iter()
        .map(|w| w.total_tss)
        .fold(1.0_f64, f64::max);
    let spark_data: Vec<u64> = app
        .weekly_loads
        .iter()
        .map(|w| ((w.total_tss / max_w) * 100.0).round().max(1.0) as u64)
        .collect();

    let n = app.weekly_loads.len().max(1);
    let mark_width = split[0].width as usize;
    let marker_line = weekly_focus_marker_line(n, week_i, mark_width);

    let mark_style = Style::default()
        .fg(Color::LightYellow)
        .add_modifier(Modifier::BOLD);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(marker_line.clone(), mark_style))),
        split[0],
    );

    let spark = Sparkline::default()
        .data(&spark_data)
        .style(Style::default().fg(Color::LightYellow))
        .max(100);
    frame.render_widget(spark, split[1]);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(marker_line, mark_style))),
        split[2],
    );
}

/// Build a line of spaces with `*` under the focused week column (and `·` ticks if sparse).
fn weekly_focus_marker_line(n_weeks: usize, week_i: usize, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let n = n_weeks.max(1);
    let mut marks = vec![' '; width];
    let pos = ((week_i as f64 + 0.5) * (width as f64) / (n as f64)) as usize;
    let pos = pos.min(width - 1);
    marks[pos] = '*';
    // Only add · ticks when weeks are few enough to read
    if n <= width / 2 {
        for wi in 0..n {
            if wi == week_i {
                continue;
            }
            let p = ((wi as f64 + 0.5) * (width as f64) / (n as f64)) as usize;
            let p = p.min(width - 1);
            if marks[p] == ' ' {
                marks[p] = '·';
            }
        }
    }
    marks.into_iter().collect()
}

fn render_metrics_view(frame: &mut Frame, app: &App, area: Rect) {
    let rows = &app.catalog_activity_metrics;
    if rows.is_empty() {
        let empty = Paragraph::new(
            "No activity metrics loaded.\n\n\
             • Press r to reload $VELOFIT_HOME/db/loadsym.sqlite\n\
             • Run: symload ingest  (then r here)\n\n\
             Table + trend / bi-plot charts once catalog has rides.",
        )
        .block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(" 4. Metrics / Library — empty "),
        );
        frame.render_widget(empty, area);
        return;
    }

    let n = rows.len();
    let focus = app.metrics_scroll.min(n.saturating_sub(1));

    let outer = Layout::vertical([
        Constraint::Length(3),      // header
        Constraint::Percentage(42), // table
        Constraint::Min(8),         // chart
        Constraint::Length(1),      // footer
    ])
    .split(area);

    let r = &rows[focus];
    let name = r.source_file.rsplit('/').next().unwrap_or(&r.source_file);
    let mode_s = match app.metrics_chart_mode {
        MetricsChartMode::Trend => format!("trend Y={}", app.metrics_trend_field.label()),
        MetricsChartMode::Biplot => format!(
            "biplot X={} Y={}",
            app.metrics_biplot_x.label(),
            app.metrics_biplot_y.label()
        ),
    };
    let hdr = Paragraph::new(vec![
        Line::from(format!(
            "[{}] ride {}/{}  {}  ·  v chart mode  1–8 field  x/y axes",
            if app.loadsym_from_catalog {
                "catalog"
            } else {
                "—"
            },
            focus + 1,
            n,
            mode_s,
        )),
        Line::from(Span::styled(
            format!(
                "{}  {}  TSLi={}  SEPi={}  {}",
                r.ride_date,
                truncate_str(name, 28),
                r.tss
                    .map(|v| format!("{:.0}", v))
                    .unwrap_or_else(|| "-".into()),
                r.np_w
                    .map(|v| format!("{:.0}", v))
                    .unwrap_or_else(|| "-".into()),
                r.sport.as_deref().unwrap_or("-"),
            ),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
    ])
    .block(
        Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .title(" 4. Metrics "),
    );
    frame.render_widget(hdr, outer[0]);

    // --- Table (compact) ---
    let h = outer[1].height.saturating_sub(2) as usize;
    let body_h = h.max(3);
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        format!(
            "  {:<10} {:>5} {:>4} {:>5} {:>5} {:>5} {:>5} {:>5} {:>4}",
            "date", "min", "spt", "avgW", "SEPi", "SRIi", "TSLi", "aHR", "FTP"
        ),
        Style::default().fg(Color::DarkGray),
    )));

    let display_order: Vec<usize> = (0..n).rev().collect();
    let focus_disp = display_order.iter().position(|&i| i == focus).unwrap_or(0);
    let start = focus_disp.saturating_sub(body_h.saturating_sub(2) / 3);
    let end = (start + body_h.saturating_sub(1)).min(display_order.len());

    for di in start..end {
        let i = display_order[di];
        let row = &rows[i];
        let marker = if i == focus { "▶" } else { " " };
        let mins = row.duration_s / 60.0;
        let sport = row
            .sport
            .as_deref()
            .map(|s| truncate_str(s, 4))
            .unwrap_or_else(|| "-".into());
        let fmt0 = |o: Option<f64>| -> String {
            o.map(|v| format!("{:.0}", v)).unwrap_or_else(|| "-".into())
        };
        let fmt1 = |o: Option<f64>| -> String {
            o.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "-".into())
        };
        let line = format!(
            "{marker} {:<10} {:>5.0} {:>4} {:>5} {:>5} {:>5} {:>5} {:>4} {:>4}",
            truncate_str(&row.ride_date, 10),
            mins,
            sport,
            fmt0(row.avg_power_w),
            fmt0(row.np_w),
            fmt1(row.intensity_factor),
            fmt0(row.tss),
            fmt0(row.avg_hr_bpm),
            fmt0(row.ftp_used_w),
        );
        let style = if i == focus {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(line, style)));
    }

    let table = Paragraph::new(lines).block(
        Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .title(" activities (newest first) "),
    );
    frame.render_widget(table, outer[1]);

    // --- Chart ---
    match app.metrics_chart_mode {
        MetricsChartMode::Trend => {
            render_metrics_trend_chart(frame, outer[2], rows, focus, app.metrics_trend_field);
        }
        MetricsChartMode::Biplot => {
            render_metrics_biplot(
                frame,
                outer[2],
                rows,
                focus,
                app.metrics_biplot_x,
                app.metrics_biplot_y,
            );
        }
    }

    let foot = match app.metrics_chart_mode {
        MetricsChartMode::Trend => {
            "↑↓ row  Enter open  v bi-plot  1–8 Y-metric  r reload  Esc list"
        }
        MetricsChartMode::Biplot => {
            "↑↓ row  Enter open  v trend  x/X axis  y/Y axis  1–8 set Y  Esc list"
        }
    };
    frame.render_widget(
        Paragraph::new(foot).style(Style::default().fg(Color::DarkGray)),
        outer[3],
    );
}

/// Clamp catalog metric values to non-negative (negatives are noise / N/A for LOADsym).
#[inline]
fn clamp_metric_nonneg(v: f64) -> f64 {
    if v.is_finite() {
        v.max(0.0)
    } else {
        0.0
    }
}

/// Axis max with small headroom; floor at least 1 so a lone 0 still shows a scale.
fn axis_max_from(values: impl Iterator<Item = f64>) -> f64 {
    let mut m = 0.0_f64;
    for v in values {
        if v.is_finite() {
            m = m.max(v);
        }
    }
    if m < 1e-12 {
        1.0
    } else {
        m * 1.05
    }
}

/// Tick labels at 0, mid, max for a [0, max] axis.
fn axis_ticks_0_max(max: f64) -> Vec<Line<'static>> {
    let mid = max / 2.0;
    vec![
        Line::from("0"),
        Line::from(format!("{mid:.0}")),
        Line::from(format!("{max:.0}")),
    ]
}

/// Trend: Y = selected field (≥0), X = ride index from 0 (oldest → newest).
fn render_metrics_trend_chart(
    frame: &mut Frame,
    area: Rect,
    rows: &[ActivityMetricsUiRow],
    focus: usize,
    field: MetricsField,
) {
    let mut data: Vec<(f64, f64)> = Vec::new();
    let mut focus_pt: Option<(f64, f64)> = None;
    for (i, row) in rows.iter().enumerate() {
        if let Some(y) = field.value(row) {
            let y = clamp_metric_nonneg(y);
            let pt = (i as f64, y);
            if i == focus {
                focus_pt = Some(pt);
            }
            data.push(pt);
        }
    }

    if data.is_empty() {
        frame.render_widget(
            Paragraph::new(format!("No finite values for {}", field.label())).block(
                Block::new()
                    .borders(Borders::ALL)
                    .padding(Padding::horizontal(1))
                    .title(format!(" Trend · {} vs ride # ", field.label())),
            ),
            area,
        );
        return;
    }

    // X and Y always origin at 0 with ticks at 0 / mid / max.
    let x_max = (rows.len().saturating_sub(1) as f64).max(1.0);
    let y_max = axis_max_from(data.iter().map(|p| p.1));

    let mut datasets = vec![Dataset::default()
        .name(field.label())
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::LightYellow))
        .data(&data)];

    let focus_data: Vec<(f64, f64)> = focus_pt.into_iter().collect();
    if !focus_data.is_empty() {
        datasets.push(
            Dataset::default()
                .name("focus")
                .marker(symbols::Marker::Dot)
                .graph_type(GraphType::Scatter)
                .style(Style::default().fg(Color::Cyan))
                .data(&focus_data),
        );
    }

    let chart = Chart::new(datasets)
        .block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(format!(
                    " Trend · {} vs ride order (old→new) · cyan = focus ",
                    field.axis_label()
                ))
                .border_style(Style::default().fg(Color::LightYellow)),
        )
        .x_axis(
            Axis::default()
                .title("Ride index (0 = oldest)")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, x_max])
                .labels(vec![
                    Line::from("0"),
                    Line::from(format!("{:.0}", x_max / 2.0)),
                    Line::from(format!("{x_max:.0}")),
                ]),
        )
        .y_axis(
            Axis::default()
                .title(field.axis_label())
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, y_max])
                .labels(axis_ticks_0_max(y_max)),
        );
    frame.render_widget(chart, area);
}

/// Bi-plot: scatter of X field vs Y field; axes from 0; negatives clamped.
fn render_metrics_biplot(
    frame: &mut Frame,
    area: Rect,
    rows: &[ActivityMetricsUiRow],
    focus: usize,
    x_field: MetricsField,
    y_field: MetricsField,
) {
    let mut cloud: Vec<(f64, f64)> = Vec::new();
    let mut focus_pt: Option<(f64, f64)> = None;
    for (i, row) in rows.iter().enumerate() {
        let (Some(x), Some(y)) = (x_field.value(row), y_field.value(row)) else {
            continue;
        };
        let pt = (clamp_metric_nonneg(x), clamp_metric_nonneg(y));
        if i == focus {
            focus_pt = Some(pt);
        }
        cloud.push(pt);
    }

    if cloud.is_empty() {
        frame.render_widget(
            Paragraph::new(format!(
                "No paired values for {} vs {}",
                x_field.label(),
                y_field.label()
            ))
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .padding(Padding::horizontal(1))
                    .title(" Bi-plot "),
            ),
            area,
        );
        return;
    }

    let x_max = axis_max_from(cloud.iter().map(|p| p.0));
    let y_max = axis_max_from(cloud.iter().map(|p| p.1));

    let focus_data: Vec<(f64, f64)> = focus_pt.into_iter().collect();
    let mut datasets = vec![Dataset::default()
        .name("rides")
        .marker(symbols::Marker::Dot)
        .graph_type(GraphType::Scatter)
        .style(Style::default().fg(Color::LightMagenta))
        .data(&cloud)];
    if !focus_data.is_empty() {
        datasets.push(
            Dataset::default()
                .name("focus")
                .marker(symbols::Marker::Block)
                .graph_type(GraphType::Scatter)
                .style(Style::default().fg(Color::Cyan))
                .data(&focus_data),
        );
    }

    let chart = Chart::new(datasets)
        .block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(format!(
                    " Bi-plot · {}  ×  {} · cyan = focus ",
                    y_field.axis_label(),
                    x_field.axis_label()
                ))
                .border_style(Style::default().fg(Color::LightMagenta)),
        )
        .x_axis(
            Axis::default()
                .title(x_field.axis_label())
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, x_max])
                .labels(axis_ticks_0_max(x_max)),
        )
        .y_axis(
            Axis::default()
                .title(y_field.axis_label())
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, y_max])
                .labels(axis_ticks_0_max(y_max)),
        );
    frame.render_widget(chart, area);
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn render_optimization_view(frame: &mut Frame, app: &App, area: Rect) {
    let loads = &app.daily_loads;
    let params = PulseResponseParams::pmc_defaults();
    let horizon = app.loadsym_plan_horizon.clamp(2, MAX_HORIZON_DAYS);
    let goal = app.loadsym_plan_goal;

    // Outer: goal banner | metrics | body | dual charts (load + form) | footer
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // goal tabs
            Constraint::Length(4), // metrics from daily summary
            Constraint::Min(6),    // plan body
            Constraint::Length(9), // two hist|proj bars: Load + Readiness
            Constraint::Length(1), // keys
        ])
        .split(area);

    // --- Goal banner: three equal columns ---
    let goal_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(outer[0]);

    let goals = [
        (LoadGoal::Recovery, "1  Recovery", goal_cols[0]),
        (LoadGoal::Maintenance, "2  Maintenance", goal_cols[1]),
        (LoadGoal::Overload, "3  Overload", goal_cols[2]),
    ];
    for (g, label, rect) in goals {
        let selected = g == goal;
        let style = if selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let title = if selected {
            format!(" ▶ {} ", label)
        } else {
            format!("   {} ", label)
        };
        // Fixed-width centering via Paragraph + full cell block so columns stay stable.
        let p = Paragraph::new(Line::from(Span::styled(title, style)))
            .alignment(ratatui::layout::Alignment::Center)
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .padding(Padding::horizontal(1))
                    .border_style(if selected {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }),
            );
        frame.render_widget(p, rect);
    }

    // --- Metrics strip (same daily TSLi series as Calendar) ---
    let mut metric_lines: Vec<Line> = Vec::new();
    if loads.is_empty() {
        metric_lines.push(Line::from(Span::styled(
            "No daily loads loaded — same series as Calendar.  r: catalog  g: 28d demo",
            Style::default().fg(Color::Yellow),
        )));
        metric_lines.push(Line::from(Span::styled(
            "Planner uses app.daily_loads (catalog total_tss / day).",
            Style::default().fg(Color::DarkGray),
        )));
        frame.render_widget(
            Paragraph::new(metric_lines).block(
                Block::new()
                    .borders(Borders::ALL)
                    .padding(Padding::horizontal(1))
                    .title(" Daily summary input "),
            ),
            outer[1],
        );
        frame.render_widget(
            Paragraph::new(Span::styled(
                " Press r or g to load data, then 1/2/3 to plan. Esc list. ",
                Style::default().fg(Color::DarkGray),
            )),
            outer[2],
        );
        render_empty_opt_charts(frame, outer[3]);
        frame.render_widget(
            Paragraph::new(Span::styled(
                " 1/2/3 goal  −/+ H (2..=10)  Enter recompute  r catalog  g demo  Esc ",
                Style::default().fg(Color::DarkGray),
            )),
            outer[4],
        );
        return;
    }

    // Plan is cached in App (recomputed on goal/horizon/data change or Enter).
    // Do not call optimize_load_plan every frame — O(7^H) would freeze the TUI.
    let hist = simulate_pulse_response(loads, &params, None).ok();
    let state = hist.as_ref().and_then(|h| h.last_state());
    let snap = compute_acute_chronic(loads, 7, 28).ok();
    let acwr = snap.as_ref().map(|s| s.acwr).unwrap_or(0.0);
    let risk = classify_acwr(acwr);
    let mono = compute_monotony(loads).unwrap_or(1.0);

    // Week / range stats from the same daily series (Calendar data)
    let n = loads.len();
    let last7: f64 = loads.iter().rev().take(7).sum();
    let last28: f64 = loads.iter().rev().take(28).sum();
    let date_lo = app
        .daily_load_dates
        .first()
        .map(|s| s.as_str())
        .unwrap_or("?");
    let date_hi = app
        .daily_load_dates
        .last()
        .map(|s| s.as_str())
        .unwrap_or("?");
    let src = if app.loadsym_from_catalog {
        "catalog"
    } else {
        "demo"
    };
    let src_path = app
        .loadsym_catalog_path
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("-");

    // Fixed-width fields so goal tabbing does not reflow this strip.
    let (ctl, atl, tsb) =
        state
            .map(|s| (s.ctl(), s.atl(), s.tsb()))
            .unwrap_or((f64::NAN, f64::NAN, f64::NAN));
    metric_lines.push(Line::from(format!(
        " source={:<7}  days={:<4}  range={:<10} → {:<10}  file={:<16}  H={:<2}d",
        src,
        n,
        truncate_str(date_lo, 10),
        truncate_str(date_hi, 10),
        truncate_str(src_path, 16),
        horizon
    )));
    metric_lines.push(Line::from(format!(
        " LTSLi={:>6.0}  STSLi={:>6.0}  SLBi={:>+7.1}  |  ACLi={:>5.2} ({:<9})  mono={:>5.2}  |  7d TSLi={:>6.0}  28d TSLi={:>7.0}",
        ctl,
        atl,
        tsb,
        acwr,
        risk.as_str(),
        mono,
        last7,
        last28
    )));
    if !app.loadsym_goal_suggest_note.is_empty() {
        let note_style = if app.loadsym_goal_user_override {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Green)
        };
        let prefix = if app.loadsym_goal_user_override {
            "override · "
        } else {
            "auto · "
        };
        metric_lines.push(Line::from(Span::styled(
            format!("{prefix}{}", app.loadsym_goal_suggest_note),
            note_style,
        )));
    }

    frame.render_widget(
        Paragraph::new(metric_lines).block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(" Daily summary → planner input (same TSLi series as Calendar) "),
        ),
        outer[1],
    );

    // --- Plan body ---
    let mut lines: Vec<Line> = Vec::new();
    // Recent history window shared by both charts (always shown).
    let hist_n = 21usize.min(loads.len());
    let hist_start = loads.len().saturating_sub(hist_n);
    let load_hist: Vec<f64> = loads[hist_start..].to_vec();
    let form_hist: Vec<f64> = hist
        .as_ref()
        .map(|h| {
            let start = h.form.len().saturating_sub(hist_n);
            h.form[start..].to_vec()
        })
        .unwrap_or_else(|| vec![0.0; load_hist.len()]);

    if let Some(plan) = app.loadsym_cached_plan.as_ref() {
        let badge = if plan.success {
            Span::styled(
                "SUCCESS",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                "FAIL/partial",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        };
        lines.push(Line::from(vec![
            Span::raw(format!(" {:<12} ", goal.as_str())),
            badge,
            Span::raw(format!(
                "   H={:<2}  load={:>5.0}%C  mean={:>6.0}  C≈{:>6.0}  form {:>+6.1}→{:>+6.1}",
                plan.daily_tss.len(),
                plan.load_frac * 100.0,
                plan.mean_plan_load,
                plan.chronic_ref,
                plan.form_start,
                plan.form_end
            )),
        ]));
        if let Some(a) = plan.projected_acwr {
            lines.push(Line::from(Span::styled(
                format!(
                    " ACLi context (advisory): projected={:.2} — does not drive SUCCESS",
                    a
                ),
                Style::default().fg(Color::DarkGray),
            )));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(" {:<6}{:>8}{:>12}", "Day", "TSLi", "form"),
            Style::default().fg(Color::DarkGray),
        )));
        for (i, tss) in plan.daily_tss.iter().enumerate() {
            let form_i = plan
                .predicted_states
                .form
                .get(i)
                .copied()
                .unwrap_or(f64::NAN);
            lines.push(Line::from(format!(
                " +{:<5}{:>8.0}{:>+12.1}",
                i + 1,
                tss,
                form_i
            )));
        }
        lines.push(Line::from(""));
        for m in plan.messages.iter().take(4) {
            lines.push(Line::from(format!("  • {}", m)));
        }
        if plan.messages.len() > 4 {
            lines.push(Line::from(format!("  … +{} more", plan.messages.len() - 4)));
        }

        render_opt_dual_charts(
            frame,
            outer[3],
            &load_hist,
            &plan.daily_tss,
            &form_hist,
            &plan.predicted_states.form,
        );
    } else {
        let err = app
            .loadsym_cached_plan_err
            .as_deref()
            .unwrap_or("No plan yet — set H with −/+, pick goal 1/2/3, or Enter to recompute");
        lines.push(Line::from(Span::styled(
            format!("Plan: {}", err),
            Style::default().fg(Color::Red),
        )));
        lines.push(Line::from(
            "Need ≥7 daily loads. Catalog (r) or demo (g). Charts still show history.",
        ));
        let empty: Vec<f64> = vec![];
        render_opt_dual_charts(frame, outer[3], &load_hist, &empty, &form_hist, &empty);
    }

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(" Recommended load (next days) "),
        ),
        outer[2],
    );

    frame.render_widget(
        Paragraph::new(Span::styled(
            " yellow=history  purple=projected · auto goal from form/fatigue · 1/2/3 override  −/+ H  Enter  r/g  Esc ",
            Style::default().fg(Color::DarkGray),
        )),
        outer[4],
    );
}

/// Empty dual-chart placeholder (no loads yet).
fn render_empty_opt_charts(frame: &mut Frame, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .border_style(Style::default().fg(Color::LightYellow))
            .title(" Load (TSLi)  ·  yellow=history | purple=projected "),
        rows[0],
    );
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .border_style(Style::default().fg(Color::LightYellow))
            .title(" Readiness (SLBi)  ·  yellow=history | purple=projected "),
        rows[1],
    );
}

/// Two horizontal bars: Load TSLi and Readiness SLBi.
/// Each bar is history (yellow, calendar-style) | projected (purple).
fn render_opt_dual_charts(
    frame: &mut Frame,
    area: Rect,
    load_hist: &[f64],
    load_proj: &[f64],
    form_hist: &[f64],
    form_proj: &[f64],
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Scale from **history only** so yellow bars do not jump when the goal
    // (and thus projected TSLi/form) changes. Projected may clip at 100 if it
    // exceeds the historical max — that is intentional for a stable past.
    let load_max = load_hist
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .fold(1.0_f64, f64::max)
        .max(1.0);
    let form_max = form_hist
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .map(form_to_spark_f)
        .fold(1.0_f64, f64::max)
        .max(1.0);

    render_hist_proj_bar(frame, rows[0], " Load (TSLi) ", load_hist, load_proj, |v| {
        ((v / load_max) * 100.0).round().clamp(0.0, 100.0) as u64
    });
    render_hist_proj_bar(
        frame,
        rows[1],
        " Readiness (SLBi) ",
        form_hist,
        form_proj,
        |v| {
            let s = form_to_spark_f(v);
            ((s / form_max) * 100.0).round().clamp(0.0, 100.0) as u64
        },
    );
}

/// One metric row: [ history sparkline yellow | projected sparkline purple ].
fn render_hist_proj_bar<F>(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    hist: &[f64],
    proj: &[f64],
    to_u64: F,
) where
    F: Fn(f64) -> u64,
{
    let n_h = hist.len().max(1);
    let n_p = proj.len().max(0);
    let n_tot = (n_h + n_p.max(1)).max(1);
    // Proportional width; keep a visible projected pane even if empty.
    let hist_pct = ((n_h * 100) / n_tot).clamp(40, 85) as u16;
    let proj_pct = 100u16.saturating_sub(hist_pct).max(15);

    let block = Block::new()
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1))
        .border_style(Style::default().fg(Color::LightYellow))
        .title(format!(
            "{} · yellow=history ({}d) | purple=projected ({}d) ",
            title.trim(),
            hist.len(),
            proj.len()
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 4 || inner.height == 0 {
        return;
    }

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(hist_pct),
            Constraint::Length(1), // divider
            Constraint::Percentage(proj_pct),
        ])
        .split(inner);

    let hist_data: Vec<u64> = hist.iter().map(|&v| to_u64(v)).collect();
    let proj_data: Vec<u64> = if proj.is_empty() {
        vec![0]
    } else {
        proj.iter().map(|&v| to_u64(v)).collect()
    };

    let hist_spark = Sparkline::default()
        .data(&hist_data)
        .style(Style::default().fg(Color::LightYellow))
        .max(100);
    frame.render_widget(hist_spark, cols[0]);

    // Visual split between history and projection
    frame.render_widget(
        Paragraph::new(Span::styled("│", Style::default().fg(Color::DarkGray))),
        cols[1],
    );

    let proj_spark = Sparkline::default()
        .data(&proj_data)
        .style(Style::default().fg(Color::Magenta))
        .max(100);
    frame.render_widget(proj_spark, cols[2]);
}

/// Shift form so negatives display; returns continuous f64 for shared scaling.
fn form_to_spark_f(form: f64) -> f64 {
    if !form.is_finite() {
        return 0.0;
    }
    (form + 100.0).clamp(0.0, 400.0)
}

// best-window logic now lives in symworx-loadsym::load::highest_rolling (and find_exceedance_regions)
