// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

use ratatui::{
    layout::Rect,
    style::{
        Color,
        Style,
    },
    symbols,
    text::Line,
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

use crate::app::{
    ResidualPanelMode,
    StatsLabResult,
};

pub fn render_fit_panel(frame: &mut Frame, area: Rect, r: &StatsLabResult) {
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

    let x_min = pts.iter().chain(line.iter()).map(|p| p.0).fold(f64::INFINITY, f64::min);
    let mut x_max = pts
        .iter()
        .chain(line.iter())
        .map(|p| p.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let y_min = pts.iter().chain(line.iter()).map(|p| p.1).fold(f64::INFINITY, f64::min);
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
        .x_axis(Axis::default().title(x_lab).bounds([x0, x_max]).labels(vec![
            Line::from(format!("{x0:.1}")),
            Line::from(format!("{:.1}", (x0 + x_max) / 2.0)),
            Line::from(format!("{x_max:.1}")),
        ]))
        .y_axis(Axis::default().title(y_lab).bounds([y0, y_max]).labels(vec![
            Line::from(format!("{y0:.1}")),
            Line::from(format!("{:.1}", (y0 + y_max) / 2.0)),
            Line::from(format!("{y_max:.1}")),
        ]));
    frame.render_widget(chart, area);
}

pub fn render_residual_panel(frame: &mut Frame, area: Rect, r: &StatsLabResult, mode: ResidualPanelMode) {
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
            let pts: Vec<(f64, f64)> = ba_mean.iter().zip(residuals.iter()).map(|(&m, &e)| (m, e)).collect();
            if pts.is_empty() {
                let pts: Vec<(f64, f64)> = residuals.iter().enumerate().map(|(i, &e)| (i as f64, e)).collect();
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

pub fn render_ba_chart(frame: &mut Frame, area: Rect, pts: &[(f64, f64)], x_lab: &str, y_lab: &str) {
    let x_min = pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let x_max = pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
    let y_abs = pts.iter().map(|p| p.1.abs()).fold(0.0_f64, f64::max).max(1e-6);
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
                .title(format!(" Bland–Altman · mean e={mean_e:.3}  ±1.96 sd  h=hist "))
                .border_style(Style::default().fg(Color::LightMagenta)),
        )
        .x_axis(Axis::default().title(x_lab).bounds([x0, x1]).labels(vec![
            Line::from(format!("{x0:.1}")),
            Line::from(format!("{:.1}", (x0 + x1) / 2.0)),
            Line::from(format!("{x1:.1}")),
        ]))
        .y_axis(Axis::default().title(y_lab).bounds([-y_lim, y_lim]).labels(vec![
            Line::from(format!("{:.1}", -y_lim)),
            Line::from("0"),
            Line::from(format!("{y_lim:.1}")),
        ]));
    frame.render_widget(chart, area);
}

pub fn render_residual_hist(frame: &mut Frame, area: Rect, residuals_data: &[f64]) {
    use symworx_stats::{
        hist_kde_with,
        HistogramConfig,
        KdeConfig,
    };

    if residuals_data.is_empty() {
        return;
    }

    // Data transforms live in symworx-stats; TUI only plots.
    let hk = hist_kde_with(
        residuals_data,
        &HistogramConfig { n_bins: 24 },
        &KdeConfig {
            n_points: 80,
            pad_frac: 0.05,
            ..Default::default()
        },
    );
    if hk.hist.n == 0 {
        return;
    }

    let bin_pts = hk.hist.centers_counts();
    let kde_pts = hk.kde_counts.clone();
    let max_c = hk.hist.max_count() as f64;
    let kde_max = kde_pts.iter().map(|p| p.1).fold(0.0_f64, f64::max);
    let y_max = max_c.max(kde_max) * 1.12;

    let x0 = hk.kde.x.first().copied().unwrap_or(hk.hist.data_min);
    let x1 = hk.kde.x.last().copied().unwrap_or(hk.hist.data_max);

    let zero_line: Vec<(f64, f64)> = if x0 <= 0.0 && x1 >= 0.0 {
        vec![(0.0, 0.0), (0.0, y_max)]
    } else {
        vec![]
    };

    let mut datasets = vec![
        Dataset::default()
            .name("bins")
            .marker(symbols::Marker::Block)
            .graph_type(GraphType::Scatter)
            .style(Style::default().fg(Color::LightYellow))
            .data(&bin_pts),
        Dataset::default()
            .name("polygon")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Yellow))
            .data(&bin_pts),
        Dataset::default()
            .name("kde")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&kde_pts),
    ];
    if zero_line.len() == 2 {
        datasets.push(
            Dataset::default()
                .name("zero")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::DarkGray))
                .data(&zero_line),
        );
    }

    let chart = Chart::new(datasets)
        .block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(" Residuals · hist + polygon + KDE  ·  h=BA ")
                .border_style(Style::default().fg(Color::LightYellow)),
        )
        .x_axis(Axis::default().title("residual y−ŷ").bounds([x0, x1]).labels(vec![
            Line::from(format!("{x0:.2}")),
            Line::from("0"),
            Line::from(format!("{x1:.2}")),
        ]))
        .y_axis(Axis::default().title("count").bounds([0.0, y_max]).labels(vec![
            Line::from("0"),
            Line::from(format!("{:.0}", y_max / 2.0)),
            Line::from(format!("{y_max:.0}")),
        ]));
    frame.render_widget(chart, area);
}
