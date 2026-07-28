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
        Chart,
        Dataset,
        GraphType,
        Padding,
        Paragraph,
    },
    Frame,
};

use super::util::truncate_str;
use crate::app::{
    ActivityMetricsUiRow,
    App,
    MetricsChartMode,
    MetricsField,
};

pub fn render_metrics_view(frame: &mut Frame, app: &App, area: Rect) {
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
            if app.loadsym_from_catalog { "catalog" } else { "—" },
            focus + 1,
            n,
            mode_s,
        )),
        Line::from(Span::styled(
            format!(
                "{}  {}  TSLi={}  SEPi={}  {}",
                r.ride_date,
                truncate_str(name, 28),
                r.tss.map(|v| format!("{:.0}", v)).unwrap_or_else(|| "-".into()),
                r.np_w.map(|v| format!("{:.0}", v)).unwrap_or_else(|| "-".into()),
                r.sport.as_deref().unwrap_or("-"),
            ),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
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
        let fmt0 = |o: Option<f64>| -> String { o.map(|v| format!("{:.0}", v)).unwrap_or_else(|| "-".into()) };
        let fmt1 = |o: Option<f64>| -> String { o.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "-".into()) };
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
            render_metrics_biplot(frame, outer[2], rows, focus, app.metrics_biplot_x, app.metrics_biplot_y);
        }
    }

    let foot = match app.metrics_chart_mode {
        MetricsChartMode::Trend => "↑↓ row  Enter open  v bi-plot  1–8 Y-metric  r reload  Esc list",
        MetricsChartMode::Biplot => "↑↓ row  Enter open  v trend  x/X axis  y/Y axis  1–8 set Y  Esc list",
    };
    frame.render_widget(
        Paragraph::new(foot).style(Style::default().fg(Color::DarkGray)),
        outer[3],
    );
}

/// Clamp catalog metric values to non-negative (negatives are noise / N/A for LOADsym).
#[inline]
pub fn clamp_metric_nonneg(v: f64) -> f64 {
    if v.is_finite() {
        v.max(0.0)
    } else {
        0.0
    }
}

/// Axis max with small headroom; floor at least 1 so a lone 0 still shows a scale.
pub fn axis_max_from(values: impl Iterator<Item = f64>) -> f64 {
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
pub fn axis_ticks_0_max(max: f64) -> Vec<Line<'static>> {
    let mid = max / 2.0;
    vec![
        Line::from("0"),
        Line::from(format!("{mid:.0}")),
        Line::from(format!("{max:.0}")),
    ]
}

/// Trend: Y = selected field (≥0), X = ride index from 0 (oldest → newest).
pub fn render_metrics_trend_chart(
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
pub fn render_metrics_biplot(
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
