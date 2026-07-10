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
    text::{
        Line,
        Span,
    },
    widgets::{
        Block,
        Borders,
        Cell,
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
    exceedance_marker_string,
    find_exceedance_regions,
    highest_rolling,
    ride_load_from_metrics,
    RiskLevel,
};

use crate::app::{
    App,
    LoadSymView,
};

/// Render the LoadSym tab (now a proper selector + sub-views per plan).
pub fn render_loadsym_tab(frame: &mut Frame, app: &App, area: Rect) {
    if app.help_mode {
        let help = Paragraph::new(
            "LoadSym help (M-? or Esc to close)\n\n\
             • ↑↓ or 1/2/3 : select sub-view from List\n\
             • In sub-views: arrows/letters for nav/scroll (sparkline), Esc to List\n\
             • Uses real ACWR / monotony / strain + NP/TSS from symworx-loadsym\n\n\
             1) Workout — i/a to load newest .fit/CSV from ~/velofit (raw|inbox) or ./data.\n\
                NP/TSS (set FTP with f/F). Thresh regions (t/T d/D). Best efforts + exceedance bars.\n\
             2) Calendar View — rolling loads + ACWR trend + scroll\n\
             3) Programming Optimization — recs based on current risk/monotony\n\n\
             Archive: ~/velofit (syncd velofit ↔ s3:bitterbeta-useast1-velofit).\n\
             Real SRM/Garmin/Polar .fit supported (power preferred for TSS)."
        ).block(Block::new().borders(Borders::ALL).title(" Help — LoadSym "));
        frame.render_widget(help, area);
        return;
    }

    let outer = Block::new()
        .title(" LoadSym — Training Load, ACWR, Optimization ")
        .borders(Borders::ALL)
        .border_style(Color::Yellow);

    let inner = outer.inner(area);
    frame.render_widget(outer, area);

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
    }
}

fn render_loadsym_list(frame: &mut Frame, app: &App, area: Rect) {
    let sel = app.loadsym_selection;

    let lines = vec![
        Line::from(Span::styled(
            "Select view (↑↓ or 1/2/3, Enter):",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
        Line::from(format!(
            "{}1. Workout Analysis",
            if sel == 0 { "▶ " } else { "  " }
        )),
        Line::from(Span::styled(
            "   Single session / ride focus: peaks, best 3/5/10/30s, threshold regions (bars)",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(format!(
            "{}2. Calendar View",
            if sel == 1 { "▶ " } else { "  " }
        )),
        Line::from(Span::styled(
            "   Multi-day load + ACWR / monotony / strain. Scroll for history.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(format!(
            "{}3. Programming Optimization",
            if sel == 2 { "▶ " } else { "  " }
        )),
        Line::from(Span::styled(
            "   Recommendations, risk alerts, load programming suggestions",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "i: load activity in Workout   g: generate demo loads   Esc or Ctrl+H: back",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let p =
        Paragraph::new(lines).block(Block::new().borders(Borders::ALL).title(" LoadSym — Home "));
    frame.render_widget(p, area);
}

fn render_workout_view(frame: &mut Frame, app: &App, area: Rect) {
    // Prefer real loaded activity (FIT / device file). Fall back to demo daily series.
    let (series, label, is_real, series_name) = if let Some(act) = &app.loaded_activity {
        let idx = app.activity_series;
        let (s, name) = if idx == 0 && act.power_w.iter().any(|v| v.is_some()) {
            (
                act.power_w
                    .iter()
                    .map(|v| v.unwrap_or(0.0))
                    .collect::<Vec<_>>(),
                "power (W)",
            )
        } else if (idx == 1 || idx == 0) && act.heart_rate_bpm.iter().any(|v| v.is_some()) {
            (
                act.heart_rate_bpm
                    .iter()
                    .map(|v| v.unwrap_or(0.0))
                    .collect::<Vec<_>>(),
                "heart rate (bpm)",
            )
        } else {
            let spd = act
                .speed_mps
                .iter()
                .map(|v| v.unwrap_or(0.0) * 3.6)
                .collect::<Vec<_>>();
            (spd, "speed (km/h)")
        };
        (
            s,
            format!("{} ({} pts)", act.source, act.times_s.len()),
            true,
            name,
        )
    } else {
        let loads = &app.daily_loads;
        let n = loads.len();
        let s: Vec<f64> = if n > 20 {
            loads[n - 20..].to_vec()
        } else {
            loads.clone()
        };
        let len = s.len();
        (s, format!("demo series ({} pts)", len), false, "demo")
    };

    let session = &series;
    let scroll = if is_real {
        app.activity_scroll
    } else {
        app.loadsym_scroll
    };

    // Visible window (scrolling viewport)
    let view_len = 60usize; // tune for terminal width feel
    let max_scroll = session.len().saturating_sub(1);
    let start = scroll
        .min(max_scroll)
        .min(session.len().saturating_sub(view_len));
    let end = (start + view_len).min(session.len());
    let visible: Vec<f64> = session[start..end].to_vec();

    let mut spark_data: Vec<u64> = vec![];
    if !visible.is_empty() {
        let minv = visible.iter().copied().fold(f64::INFINITY, f64::min);
        let maxv = visible.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let range = if maxv > minv { maxv - minv } else { 1.0 };
        spark_data = visible
            .iter()
            .map(|&v| (((v - minv) / range) * 200.0) as u64)
            .collect();
    }

    // Real analytics on the *full* loaded (or demo) series
    let best3 = highest_rolling(session, 3);
    let best5 = highest_rolling(session, 5);
    let best10 = highest_rolling(session, 10);
    let best30 = highest_rolling(session, 30.min(session.len()));

    let mean: f64 = session.iter().sum::<f64>() / session.len() as f64;
    let var = session.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / session.len() as f64;
    let std = var.sqrt();
    let peak = session.iter().copied().fold(0.0_f64, f64::max);
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
    let regions = find_exceedance_regions(session, thresh, min_dur);

    let marker = exceedance_marker_string(session, thresh);

    // New: cycling power metrics (TSS etc) when we have a real activity with power
    let ride_metrics = if is_real {
        if let Some(act) = &app.loaded_activity {
            let p = act.power_series();
            Some(compute_ride_metrics(&act.times_s, &p, app.ftp))
        } else {
            None
        }
    } else {
        None
    };

    // EVENLY TABULATED SUMMARY ACROSS THE TOP (auto-derived core stats)
    let header_row: Vec<Cell> = vec![
        "Mean", "Std", "Peak", "Best3", "Best30", "Regions", "Thresh", "NP", "TSS",
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
        format!("{}", regions.len()),
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
    .block(Block::new().borders(Borders::ALL).title(format!(
        "Workout Summary — {} [{}]  FTP={:.0}W",
        label, series_name, app.ftp
    )));

    let mut info_lines = vec![format!(
        "view: {}..{} / {}   (scroll ← →)",
        start,
        end,
        session.len()
    )];
    if is_real {
        if let Some(act) = &app.loaded_activity {
            if let Some(m) = &act.manufacturer {
                info_lines.push(format!(
                    "Device: {} {}",
                    m,
                    act.product.as_deref().unwrap_or("")
                ));
            }
            if let Some(sp) = &act.sport {
                info_lines.push(format!("Sport: {}", sp));
            }
            info_lines.push(format!(
                "Duration: {:.0}s  Samples: {}",
                act.duration_s(),
                act.len()
            ));
        }
    }
    if let Some(ref m) = ride_metrics {
        info_lines.push(format!(
            "Ride: NP={:.0}W  IF={:.2}  TSS={:.1}  Work={:.0}kJ",
            m.np, m.if_, m.tss, m.total_work_kj
        ));
    }
    info_lines.push(format!(
        "Above thresh (min_dur={}): {} regions   (│ = exceedance)",
        min_dur,
        regions.len()
    ));
    info_lines.push(format!("Markers: {}", marker));

    let info = Paragraph::new(info_lines.join("\n")).block(Block::new().borders(Borders::ALL));

    let spark = Sparkline::default()
        .data(&spark_data)
        .style(Style::default().fg(if is_real {
            Color::LightGreen
        } else {
            Color::LightCyan
        }))
        .max(200);

    let chunks = Layout::vertical([
        Constraint::Length(4), // tabulated summary table at very top
        Constraint::Length(4), // info / device / regions
        Constraint::Length(3), // sparkline
        Constraint::Length(2), // footer
    ])
    .split(area);

    frame.render_widget(summary_table, chunks[0]);
    frame.render_widget(info, chunks[1]);
    frame.render_widget(spark, chunks[2]);

    let footer = Paragraph::new("i/a: load newest .fit (~/velofit)   r:reset   ←→ scroll   1/2/3 series   t/T thresh   d/D dur   f/F: FTP+/-   Esc:list")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[3]);
}

fn render_calendar_view(frame: &mut Frame, app: &App, area: Rect) {
    let loads = &app.daily_loads;
    let scroll = app.loadsym_scroll.min(loads.len().saturating_sub(1));

    // Compute ACWR on the series (latest)
    let acwr_snap = compute_acute_chronic(loads, 7, 28).ok();
    let mono = compute_monotony(loads).unwrap_or(1.0);
    let strain = compute_strain(loads).unwrap_or(0.0);

    let risk_str = acwr_snap
        .as_ref()
        .map(|s| format!("{:?}", classify_acwr(s.acwr)))
        .unwrap_or("N/A".to_string());

    let mut lines: Vec<Line> = vec![
        Line::from(format!(
            "Calendar / Trend View — scroll offset {} / {}",
            scroll,
            loads.len()
        )),
        Line::from(format!(
            "Latest ACWR: {:.2}  Risk: {}   Monotony: {:.2}   Strain: {:.1}",
            acwr_snap.as_ref().map(|s| s.acwr).unwrap_or(0.0),
            risk_str,
            mono,
            strain
        )),
        Line::from(""),
    ];

    // Show windowed view of daily loads around scroll
    let win = 14usize;
    let start = scroll
        .saturating_sub(win / 2)
        .min(loads.len().saturating_sub(win));
    let end = (start + win).min(loads.len());
    for (i, &ld) in loads[start..end].iter().enumerate() {
        let idx = start + i;
        let marker = if idx == scroll { "▶" } else { " " };
        let ac = if let Ok(s) = compute_acute_chronic(&loads[..=idx.min(loads.len() - 1)], 7, 28) {
            s.acwr
        } else {
            0.0
        };
        lines.push(Line::from(format!(
            "{} Day {}: load={:5.0}   ACWR~{:.2}",
            marker, idx, ld, ac
        )));
    }

    let spark_data: Vec<u64> = loads.iter().map(|&v| (v / 5.0) as u64).collect(); // rough scale
    let spark = Sparkline::default()
        .data(&spark_data)
        .style(Style::default().fg(Color::LightYellow))
        .max(150);

    let chunks = Layout::vertical([Constraint::Min(10), Constraint::Length(3)]).split(area);
    let p = Paragraph::new(lines).block(
        Block::new()
            .borders(Borders::ALL)
            .title(" 2. Calendar View "),
    );
    frame.render_widget(p, chunks[0]);
    frame.render_widget(spark, chunks[1]);
}

fn render_optimization_view(frame: &mut Frame, app: &App, area: Rect) {
    let loads = &app.daily_loads;
    let snap = compute_acute_chronic(loads, 7, 28).ok();
    let acwr = snap.as_ref().map(|s| s.acwr).unwrap_or(0.0);
    let risk = classify_acwr(acwr);
    let mono = compute_monotony(loads).unwrap_or(1.0);

    let mut recs = vec![];
    match risk {
        RiskLevel::Low => {
            recs.push("Good window — can increase load 5-10% next week if recovery OK.")
        }
        RiskLevel::Moderate => {
            recs.push("Moderate risk — maintain or small +2-5% with monitoring.")
        }
        RiskLevel::High => recs.push("HIGH — reduce acute load 10-20%, focus recovery."),
        RiskLevel::VeryHigh => recs.push("VERY HIGH — deload recommended. Reassess in 3-5 days."),
    }
    if mono > 2.0 {
        recs.push("High monotony: introduce more variation (hard / easy days).");
    }

    let lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "Programming Optimization & Recommendations",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("Current ACWR: {:.2} → {:?}", acwr, risk)),
        Line::from(format!("Monotony: {:.2}", mono)),
        Line::from(""),
        Line::from("Suggestions:"),
    ]
    .into_iter()
    .chain(recs.into_iter().map(|r| Line::from(format!("  • {}", r))))
    .collect();

    let p = Paragraph::new(lines).block(
        Block::new()
            .borders(Borders::ALL)
            .title(" 3. Programming Optimization "),
    );
    frame.render_widget(p, area);
}

// best-window logic now lives in symworx-loadsym::load::highest_rolling (and find_exceedance_regions)
