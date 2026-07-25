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
        Table,
    },
    Frame,
};
use symworx_loadsym::load::{
    compute_ride_metrics,
    find_exceedance_regions,
    highest_rolling,
};

use crate::app::{
    App,
    WorkoutStream,
};

pub fn stream_color(s: WorkoutStream) -> Color {
    match s {
        WorkoutStream::Power => Color::LightGreen,
        WorkoutStream::HeartRate => Color::LightRed,
        WorkoutStream::Speed => Color::LightCyan,
        WorkoutStream::Cadence => Color::Yellow,
        WorkoutStream::Elevation => Color::White,
    }
}

pub fn stream_series(
    act: &symworx_io::ActivityData,
    s: WorkoutStream,
) -> (Vec<f64>, &'static str, bool) {
    (s.series(act), s.chart_title(), s.present_on(act))
}

/// Visible sample window for workout line charts (shared pan across panels).
const WORKOUT_VIEW_LEN: usize = 240;

pub fn render_workout_view(frame: &mut Frame, app: &App, area: Rect) {
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
pub fn render_workout_line_chart(
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
