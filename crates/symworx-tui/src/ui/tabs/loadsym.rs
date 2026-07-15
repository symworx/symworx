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
    simulate_pulse_response,
    LoadGoal,
    MAX_HORIZON_DAYS,
    PulseResponseParams,
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
             1) Workout — i/a to load newest .fit/CSV from $VELOFIT_HOME (raw|inbox) or ./data.\n\
                NP/TSS (set FTP with f/F). Thresh regions (t/T d/D). Best efforts + exceedance bars.\n\
             2) Calendar — daily TSS from personal SQLite catalog ($VELOFIT_HOME/db).\n\
                ↑↓/←→ scroll  Home/End  r:reload catalog  g:demo\n\
             3) Programming Optimization — load vs chronic bands (1/2/3 goals; ACWR is context)\n\n\
             Archive: $VELOFIT_HOME (default ~/velofit). Catalog is personal (not in git).\n\
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
            if app.loadsym_from_catalog {
                format!(
                    "   {} days from catalog  • multi-day TSS + ACWR",
                    app.daily_loads.len()
                )
            } else {
                "   Multi-day load + ACWR (r: reload catalog, g: demo)".to_string()
            },
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(format!(
            "{}3. Programming Optimization",
            if sel == 2 { "▶ " } else { "  " }
        )),
        Line::from(Span::styled(
            "   Multi-day plan: recovery 20–55%C · maintain 85–115%C · overload 115–140%C",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "i: load activity   r: reload catalog   g: demo loads   Esc/Ctrl+H: back",
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
        Constraint::Length(7), // weekly TSS: * above + bars + * below + label
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
    //   DATE       TSS      n │ WEEK       W-TSS    rides │ ACWR  risk       mono  strain
    let col_hdr = format!(
        "{:<10}  {:>7}  {:>3}  │  {:<10}  {:>7}  {:>5}  │  {:>5}  {:<9}  {:>5}  {:>7}",
        "DATE", "TSS", "n", "WEEK", "W-TSS", "rides", "ACWR", "risk", "mono", "strain"
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
        Line::from(Span::styled(
            col_hdr,
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            col_vals,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
    ])
    .block(Block::new().borders(Borders::ALL).title(" 2. Calendar "));
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
        format!(
            "  {:<10}  {:>7}  {:>3}  {:>6}",
            "date", "TSS", "n", "ACWR"
        ),
        Style::default().fg(Color::DarkGray),
    )));
    for idx in (day_lo..=day_hi).rev() {
        let marker = if idx == day_i { "▶" } else { " " };
        let label = app
            .daily_load_dates
            .get(idx)
            .cloned()
            .unwrap_or_else(|| format!("d{idx}"));
        let ac = app
            .daily_acwr
            .get(idx)
            .and_then(|a| *a)
            .unwrap_or(0.0);
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
        for (k, r) in day_rides.iter().take(ride_lines_budget.saturating_sub(1)).enumerate() {
            let name = r
                .source_file
                .rsplit('/')
                .next()
                .unwrap_or(&r.source_file);
            let mins = r.duration_s / 60.0;
            daily_lines.push(Line::from(format!(
                "  {:>2}. {:<24}  {:>7.1}  {:>5.0}m",
                k + 1,
                truncate_str(name, 24),
                r.tss,
                mins
            )));
        }
        if day_rides.len() > ride_lines_budget.saturating_sub(1) {
            daily_lines.push(Line::from(format!(
                "  … +{} more rides",
                day_rides.len() + 1 - ride_lines_budget
            )));
        }
    }

    let daily_p = Paragraph::new(daily_lines).block(
        Block::new()
            .borders(Borders::ALL)
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
                "week", "W-TSS", "rides", "days"
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
            .title(" Weekly ")
            .border_style(if app.loadsym_scroll_from_week {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            }),
    );
    frame.render_widget(week_p, cols[1]);

    // --- Bottom bar: weekly TSS aggregates + * under focused week ---
    render_weekly_tss_bar(frame, app, outer[2], week_i);
}

/// Weekly TSS sparkline across the bottom; `*` marks the focused week (above + below).
fn render_weekly_tss_bar(frame: &mut Frame, app: &App, area: Rect, week_i: usize) {
    let n_weeks = app.weekly_loads.len();
    let title = if n_weeks == 0 {
        " weekly TSS — no weeks loaded (r to reload catalog) ".to_string()
    } else if let Some(w) = app.weekly_loads.get(week_i) {
        format!(
            " weekly TSS  ·  {} weeks  ·  focus {}  TSS={:.0}  rides={}  * ",
            n_weeks, w.week_start, w.total_tss, w.ride_count
        )
    } else {
        format!(" weekly TSS  ·  {} weeks ", n_weeks)
    };
    let block = Block::new()
        .borders(Borders::ALL)
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
            Constraint::Length(9), // two hist|proj bars: Load + Form
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
                    .border_style(if selected {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }),
            );
        frame.render_widget(p, rect);
    }

    // --- Metrics strip (same daily TSS series as Calendar) ---
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
    let date_lo = app.daily_load_dates.first().map(|s| s.as_str()).unwrap_or("?");
    let date_hi = app.daily_load_dates.last().map(|s| s.as_str()).unwrap_or("?");
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
    let (ctl, atl, tsb) = state
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
        " CTL={:>6.0}  ATL={:>6.0}  TSB={:>+7.1}  |  ACWR={:>5.2} ({:<9})  mono={:>5.2}  |  7d TSS={:>6.0}  28d TSS={:>7.0}",
        ctl,
        atl,
        tsb,
        acwr,
        risk.as_str(),
        mono,
        last7,
        last28
    )));

    frame.render_widget(
        Paragraph::new(metric_lines).block(
            Block::new()
                .borders(Borders::ALL)
                .title(" Daily summary → planner input (same TSS series as Calendar) "),
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
                    " ACWR context (advisory): projected={:.2} — does not drive SUCCESS",
                    a
                ),
                Style::default().fg(Color::DarkGray),
            )));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(" {:<6}{:>8}{:>12}", "Day", "TSS", "form"),
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
            lines.push(Line::from(format!(
                "  … +{} more",
                plan.messages.len() - 4
            )));
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
                .title(" Recommended load (next days) "),
        ),
        outer[2],
    );

    frame.render_widget(
        Paragraph::new(Span::styled(
            " yellow=history  purple=projected · 1/2/3 goal  −/+ H(2–10)  Enter recompute  r/g  Esc ",
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
            .border_style(Style::default().fg(Color::LightYellow))
            .title(" Load (TSS)  ·  yellow=history | purple=projected "),
        rows[0],
    );
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightYellow))
            .title(" Form (TSB)  ·  yellow=history | purple=projected "),
        rows[1],
    );
}

/// Two horizontal bars: Load TSS and Form TSB.
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
    // (and thus projected TSS/form) changes. Projected may clip at 100 if it
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

    render_hist_proj_bar(frame, rows[0], " Load (TSS) ", load_hist, load_proj, |v| {
        ((v / load_max) * 100.0).round().clamp(0.0, 100.0) as u64
    });
    render_hist_proj_bar(frame, rows[1], " Form (TSB) ", form_hist, form_proj, |v| {
        let s = form_to_spark_f(v);
        ((s / form_max) * 100.0).round().clamp(0.0, 100.0) as u64
    });
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
        Paragraph::new(Span::styled(
            "│",
            Style::default().fg(Color::DarkGray),
        )),
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
