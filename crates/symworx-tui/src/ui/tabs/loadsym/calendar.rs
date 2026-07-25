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

use super::util::truncate_str;
use crate::app::{
    ActivityMetricsUiRow,
    App,
    LoadSymView,
    MetricsChartMode,
    MetricsField,
    WorkoutStream,
};

pub(crate) fn render_calendar_view(frame: &mut Frame, app: &App, area: Rect) {
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
pub(crate) fn render_weekly_tsli_bar(frame: &mut Frame, app: &App, area: Rect, week_i: usize) {
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
pub(crate) fn weekly_focus_marker_line(n_weeks: usize, week_i: usize, width: usize) -> String {
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
