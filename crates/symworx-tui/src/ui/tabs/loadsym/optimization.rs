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
        Padding,
        Paragraph,
        Sparkline,
    },
    Frame,
};
use symworx_loadsym::load::{
    classify_acwr,
    compute_acute_chronic,
    compute_monotony,
    simulate_pulse_response,
    LoadGoal,
    PulseResponseParams,
    MAX_HORIZON_DAYS,
};

use super::util::truncate_str;
use crate::app::App;

pub(crate) fn render_optimization_view(frame: &mut Frame, app: &App, area: Rect) {
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
pub(crate) fn render_empty_opt_charts(frame: &mut Frame, area: Rect) {
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
pub(crate) fn render_opt_dual_charts(
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
pub(crate) fn render_hist_proj_bar<F>(
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
pub(crate) fn form_to_spark_f(form: f64) -> f64 {
    if !form.is_finite() {
        return 0.0;
    }
    (form + 100.0).clamp(0.0, 400.0)
}

// best-window logic now lives in symworx-loadsym::load::highest_rolling (and find_exceedance_regions)
