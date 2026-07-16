use ratatui::{
    layout::{
        Constraint,
        Layout,
        Rect,
    },
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
        Paragraph,
    },
    Frame,
};

use crate::app::{
    App,
    ExploreView,
};

/// Visible sample window width for the Explore waveform chart.
pub const EXPLORE_VIEW_LEN: usize = 300;
/// Visible interval count for the tachogram chart.
pub const TACHO_VIEW_LEN: usize = 80;

pub fn render_explore_tab(frame: &mut Frame, app: &App, area: Rect) {
    if app.help_mode {
        let help = Paragraph::new(
            "Explore help (M-? or Esc to close)\n\n\
             WORKFLOW\n\
             1. Import: Ctrl+G Resting PPG / Respiration — biosym ground-truth peaks kept\n\
             2. Waveform: cyan series · green known primary · yellow secondary · red detected\n\
             3. k peak detect  ·  K peak params (live)  ·  p process (incl. 1st/2nd derivative)\n\
             4. Peak–peak intervals rebuild as a tachogram after each detect\n\
             5. i toggle tachogram view  ·  o source (detected vs known)  ·  e export CSV\n\
             6. Compare known vs detected (match count in status)\n\n\
             BINDINGS\n\
             • p       process menu (1–5 ops; ←→ window for MA/median)\n\
             • k       run peak detection (current params) + rebuild tachogram\n\
             • K       peak parameter editor (chart stays visible; Enter applies)\n\
             • i       toggle waveform ↔ tachogram (interval series)\n\
             • o       tachogram source: detected peaks ↔ known primary\n\
             • e       export tachogram CSV → data/tachogram_*.csv\n\
             • t / T   toggle known / detected peak overlays (waveform)\n\
             • r       reset original (clears detected peaks + tachogram)\n\
             • ← → h l pan viewport (waveform samples / tachogram intervals)\n\
             • Esc     back to Import (or close process/peak-param submode)\n\n\
             TACHOGRAM\n\
             • Y = peak–peak interval (sec when fs known, else samples)\n\
             • X = interval index (beat n → n+1)\n\
             • Rates (events/min) in export column when fs known\n\
             • Uses successive differences (symworx-math) on peak times\n\n\
             PEAK PARAMS (K)\n\
             • height_frac / prom_frac / min_interval_sec / match_tol\n\
             • ↑↓ field  ←→/± live  1–4 jump  d defaults\n\
             • k re-run (stay open)  ·  Enter apply + close  ·  Esc close\n\
             • Waveform chart stays visible under the editor so overlays update live\n\n\
             TIP: PPG → k → i (tachogram) → o (known vs detected IBI) → e export.",
        )
        .block(
            Block::new()
                .borders(Borders::ALL)
                .title(" Help — Explore (BioSym) "),
        );
        frame.render_widget(help, area);
        return;
    }

    if app.pending_peak_params {
        // Keep the waveform chart visible under the editor so live / Enter / k
        // re-detects are immediately visible (previously the full-screen panel
        // made re-run look like a no-op when peak counts were unchanged).
        if app.loaded_signal.is_some() {
            let chunks = Layout::vertical([Constraint::Length(16), Constraint::Min(6)]).split(area);
            render_peak_params_panel(frame, app, chunks[0]);
            render_waveform(frame, app, chunks[1]);
        } else {
            render_peak_params_panel(frame, app, area);
        }
        return;
    }

    if app.pending_process {
        let names = [
            "Moving Average",
            "Median Filter",
            "Detrend (mean)",
            "1st derivative (d/dt)",
            "2nd derivative (d²/dt²)",
        ];
        let mut lines = vec![
            "\n\nSignal Processing\n\n".to_string(),
            "Choose operation:\n\n".to_string(),
        ];
        for (i, name) in names.iter().enumerate() {
            let sel = if app.process_selection == i { ">" } else { " " };
            let win = if i < 2 {
                format!(" (window {})", app.process_window)
            } else {
                String::new()
            };
            lines.push(format!("{} {}{}\n", sel, name, win));
        }
        lines.push(
            "\n↑↓ or 1–5   ←→/-+ window (MA/Median)   Enter apply   Esc cancel\n\
             Derivatives use successive differences (symworx-math); length preserved.\n\
             After apply, peak detection re-runs with current K params so you can compare overlays.\n"
                .to_string(),
        );
        let content = Paragraph::new(lines.join(""))
            .block(Block::new().borders(Borders::ALL).title(" Process "));
        frame.render_widget(content, area);
        return;
    }

    if app.loaded_signal.is_none() {
        let block = Block::new()
            .title(" Explore — BioSym ")
            .borders(Borders::ALL)
            .border_style(Color::Magenta);
        let content = Paragraph::new(
            "Load or Generate BioSym signal (Import or Ctrl+G presets)\n\n\
             Resting PPG / Respiration keep generator ground-truth peaks for overlay.\n\
             After load: p process · k peak detect · K params · i tachogram · e export.\n\n\
             Chart: cyan = series; green = known primary; yellow = secondary; red = detected.\n\
             M-? for full BioSym Explore help.",
        )
        .block(block);
        frame.render_widget(content, area);
        return;
    }

    match app.explore_view {
        ExploreView::Waveform => render_waveform(frame, app, area),
        ExploreView::Tachogram => render_tachogram(frame, app, area),
    }
}

fn render_waveform(frame: &mut Frame, app: &App, area: Rect) {
    let signal = app.loaded_signal.as_ref().expect("checked");
    let block = Block::new()
        .title(" Explore — Waveform ")
        .borders(Borders::ALL)
        .border_style(Color::Magenta);

    let stats = crate::app::compute_basic_stats(&signal.current);

    let view_len = EXPLORE_VIEW_LEN;
    let n = signal.current.len();
    let max_start = n.saturating_sub(view_len);
    let start = app.explore_scroll.min(max_start);
    let end = if n == 0 { 0 } else { (start + view_len).min(n) };
    let visible: Vec<f64> = if n > 0 && start < end {
        signal.current[start..end].to_vec()
    } else {
        vec![]
    };

    let mut y_min = stats.min;
    let mut y_max = stats.max;
    if !visible.is_empty() {
        y_min = visible.iter().copied().fold(f64::INFINITY, f64::min);
        y_max = visible.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    }
    if !y_min.is_finite() || !y_max.is_finite() || (y_max - y_min).abs() < 1e-12 {
        y_min = -1.0;
        y_max = 1.0;
    }
    let pad = (y_max - y_min) * 0.05;
    y_min -= pad;
    y_max += pad;

    let data: Vec<(f64, f64)> = visible
        .iter()
        .enumerate()
        .map(|(i, &v)| ((start + i) as f64, v))
        .collect();

    let peak_pts = |idxs: &[usize]| -> Vec<(f64, f64)> {
        idxs.iter()
            .filter(|&&i| i >= start && i < end && i < signal.current.len())
            .map(|&i| (i as f64, signal.current[i]))
            .collect()
    };
    let known_primary = if signal.show_known_peaks {
        peak_pts(&signal.known_peaks_primary)
    } else {
        vec![]
    };
    let known_secondary = if signal.show_known_peaks {
        peak_pts(&signal.known_peaks_secondary)
    } else {
        vec![]
    };
    let detected = if signal.show_detected_peaks {
        peak_pts(&signal.detected_peaks)
    } else {
        vec![]
    };

    let mut datasets = vec![Dataset::default()
        .name("signal")
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::LightCyan))
        .data(&data)];

    if !known_primary.is_empty() {
        datasets.push(
            Dataset::default()
                .name(signal.kind.primary_peak_label())
                .marker(symbols::Marker::Dot)
                .graph_type(GraphType::Scatter)
                .style(Style::default().fg(Color::Green))
                .data(&known_primary),
        );
    }
    if !known_secondary.is_empty() {
        datasets.push(
            Dataset::default()
                .name(signal.kind.secondary_peak_label())
                .marker(symbols::Marker::Dot)
                .graph_type(GraphType::Scatter)
                .style(Style::default().fg(Color::Yellow))
                .data(&known_secondary),
        );
    }
    if !detected.is_empty() {
        datasets.push(
            Dataset::default()
                .name("detected")
                .marker(symbols::Marker::Block)
                .graph_type(GraphType::Scatter)
                .style(Style::default().fg(Color::LightRed))
                .data(&detected),
        );
    }

    let x_axis = Axis::default()
        .title("Sample index (←→ pan)")
        .style(Style::default().fg(Color::Gray))
        .bounds([start as f64, end as f64])
        .labels(vec![
            Line::from(format!("{}", start)),
            Line::from(format!("{}", (start + end) / 2)),
            Line::from(format!("{}", end)),
        ]);

    let y_axis = Axis::default()
        .title("Amplitude")
        .style(Style::default().fg(Color::Gray))
        .bounds([y_min, y_max])
        .labels(vec![
            Line::from(format!("{:.2}", y_min)),
            Line::from(format!("{:.2}", (y_min + y_max) / 2.0)),
            Line::from(format!("{:.2}", y_max)),
        ]);

    let chart = Chart::new(datasets)
        .block(Block::new())
        .x_axis(x_axis)
        .y_axis(y_axis);

    let pan_hint = if n > view_len {
        format!("  ←→ pan  window {}/{}", start, max_start)
    } else {
        String::new()
    };
    let fs_s = signal
        .fs
        .map(|f| format!("{:.0} Hz", f))
        .unwrap_or_else(|| "fs?".into());
    let n_ibi = signal
        .tachogram
        .as_ref()
        .map(|t| t.n_intervals())
        .unwrap_or(0);
    let pp = &app.peak_params;
    let lines = vec![
        format!(
            "File: {}  |  {}  {}  |  samples {}..{} / {}{}  |  view: waveform (i=tacho)",
            signal.name, signal.kind.label(), fs_s, start, end, n, pan_hint
        ),
        format!(
            "Mean: {:.2}  Std: {:.2}  Min: {:.2}  Max: {:.2}  |  known {}/{}  det {}  IBI n={}",
            stats.mean,
            stats.std,
            stats.min,
            stats.max,
            signal.known_peaks_primary.len(),
            signal.known_peaks_secondary.len(),
            signal.detected_peaks.len(),
            n_ibi
        ),
        format!(
            "Peak params: h={:.2} prom={:.2} min_int={:.2}s tol=±{}  |  p k K  i tacho  e export  o source",
            pp.height_frac, pp.prom_frac, pp.min_interval_sec, pp.match_tol
        ),
    ];

    let content = Paragraph::new(lines.join("\n")).block(block);
    let chunks = Layout::vertical([Constraint::Length(5), Constraint::Min(1)]).split(area);
    frame.render_widget(content, chunks[0]);
    frame.render_widget(chart, chunks[1]);
}

fn render_tachogram(frame: &mut Frame, app: &App, area: Rect) {
    let signal = app.loaded_signal.as_ref().expect("checked");
    let block = Block::new()
        .title(format!(
            " Explore — Tachogram ({}) ",
            signal.tachogram_source.label()
        ))
        .borders(Borders::ALL)
        .border_style(Color::Yellow);

    let Some(tacho) = signal.tachogram.as_ref() else {
        let msg = format!(
            "No tachogram yet for source «{}».\n\n\
             Need ≥2 peaks:\n\
             • k — detect peaks on current series, or\n\
             • o — switch to known primary (after Ctrl+G generate)\n\
             • i — return to waveform view\n\
             • e — export (after intervals exist)\n",
            signal.tachogram_source.label()
        );
        frame.render_widget(Paragraph::new(msg).block(block), area);
        return;
    };

    let n = tacho.n_intervals();
    let view_len = TACHO_VIEW_LEN;
    let max_start = n.saturating_sub(view_len);
    let start = app.explore_scroll.min(max_start);
    let end = if n == 0 { 0 } else { (start + view_len).min(n) };
    let visible = if n > 0 && start < end {
        &tacho.intervals[start..end]
    } else {
        &tacho.intervals[..0]
    };

    let mut y_min = visible.iter().copied().fold(f64::INFINITY, f64::min);
    let mut y_max = visible.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !y_min.is_finite() || !y_max.is_finite() || (y_max - y_min).abs() < 1e-12 {
        y_min = 0.0;
        y_max = 1.0;
    }
    let pad = (y_max - y_min) * 0.08;
    y_min = (y_min - pad).max(0.0);
    y_max += pad;

    let data: Vec<(f64, f64)> = visible
        .iter()
        .enumerate()
        .map(|(i, &v)| ((start + i) as f64, v))
        .collect();

    let datasets = vec![Dataset::default()
        .name("interval")
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::LightMagenta))
        .data(&data)];

    let y_title = if tacho.unit_is_sec {
        "Interval (s)"
    } else {
        "Interval (samples)"
    };
    let x_axis = Axis::default()
        .title("Interval index (←→ pan)")
        .style(Style::default().fg(Color::Gray))
        .bounds([start as f64, end.max(start + 1) as f64])
        .labels(vec![
            Line::from(format!("{}", start)),
            Line::from(format!("{}", (start + end) / 2)),
            Line::from(format!("{}", end)),
        ]);
    let y_axis = Axis::default()
        .title(y_title)
        .style(Style::default().fg(Color::Gray))
        .bounds([y_min, y_max])
        .labels(vec![
            Line::from(format!("{:.3}", y_min)),
            Line::from(format!("{:.3}", (y_min + y_max) / 2.0)),
            Line::from(format!("{:.3}", y_max)),
        ]);

    let chart = Chart::new(datasets)
        .block(Block::new())
        .x_axis(x_axis)
        .y_axis(y_axis);

    let mean = tacho
        .mean_interval()
        .map(|m| {
            if tacho.unit_is_sec {
                format!("{:.3} s", m)
            } else {
                format!("{:.1} smp", m)
            }
        })
        .unwrap_or_else(|| "n/a".into());
    let rate = tacho
        .mean_rate()
        .map(|r| format!("{:.1} /min", r))
        .unwrap_or_else(|| "n/a".into());
    let pan_hint = if n > view_len {
        format!("  ←→ pan {}/{}", start, max_start)
    } else {
        String::new()
    };
    let lines = vec![
        format!(
            "Source: {}  |  {} peaks → {} intervals{}  |  view: tachogram (i=wave)",
            tacho.source.label(),
            tacho.peak_indices.len(),
            n,
            pan_hint
        ),
        format!(
            "Mean IBI: {}  |  mean rate: {}  |  unit: {}  |  o switch source  e export CSV",
            mean,
            rate,
            if tacho.unit_is_sec {
                "seconds"
            } else {
                "samples (set fs via generate)"
            }
        ),
        format!(
            "Peaks: det={}  known_primary={}  |  k re-detect  K params  e → data/tachogram_*.csv",
            signal.detected_peaks.len(),
            signal.known_peaks_primary.len()
        ),
    ];

    let content = Paragraph::new(lines.join("\n")).block(block);
    let chunks = Layout::vertical([Constraint::Length(5), Constraint::Min(1)]).split(area);
    frame.render_widget(content, chunks[0]);
    frame.render_widget(chart, chunks[1]);
}

fn render_peak_params_panel(frame: &mut Frame, app: &App, area: Rect) {
    let pp = &app.peak_params;
    let sel = app.peak_param_selection;
    let kind = app
        .loaded_signal
        .as_ref()
        .map(|s| s.kind.label())
        .unwrap_or("?");
    let fs = app.loaded_signal.as_ref().and_then(|s| s.fs);

    let thr = app
        .loaded_signal
        .as_ref()
        .and_then(|s| crate::processing::peak_thresholds(&s.current, s.fs, pp));

    let mut lines = vec![
        format!("\n  Peak detection parameters  ({})\n", kind),
        "  Absolute thresholds recompute from the *current* series range.\n\n".to_string(),
    ];

    let fields: [(&str, String); 4] = [
        (
            "height_frac",
            format!(
                "{:.2}   (min height as frac of range above min)",
                pp.height_frac
            ),
        ),
        (
            "prom_frac",
            format!("{:.2}   (min prominence as frac of range)", pp.prom_frac),
        ),
        (
            "min_interval_sec",
            format!(
                "{:.2} s  → dist {} samples @ fs={}",
                pp.min_interval_sec,
                thr.map(|t| t.2).unwrap_or(0),
                fs.map(|f| format!("{:.0}", f))
                    .unwrap_or_else(|| "?".into())
            ),
        ),
        (
            "match_tol",
            format!("{} samples  (known-peak match window)", pp.match_tol),
        ),
    ];

    for (i, (name, val)) in fields.iter().enumerate() {
        let mark = if i == sel { ">" } else { " " };
        lines.push(format!("  {} {}. {:18} {}\n", mark, i + 1, name, val));
    }

    if let Some((h, p, d, range, ymin, ymax)) = thr {
        lines.push(format!(
            "\n  Effective on current series: height={:.4}  prom={:.4}  dist={}  range=[{:.3},{:.3}] span={:.3}\n",
            h, p, d, ymin, ymax, range
        ));
    }

    if let Some(sig) = &app.loaded_signal {
        let n_det = sig.detected_peaks.len();
        let n_known = sig.known_peaks_primary.len();
        let matched = if n_known > 0 {
            crate::processing::count_peak_matches(
                &sig.known_peaks_primary,
                &sig.detected_peaks,
                pp.match_tol,
            )
        } else {
            0
        };
        lines.push(format!(
            "  Outcome: detected={}  known_primary={}  matched={} within ±{}\n",
            n_det, n_known, matched, pp.match_tol
        ));
    }

    lines.push(
        "\n  ↑↓ field   ←→ / ± live re-detect   1–4 jump   d kind defaults\n\
           k / K re-run (stay here)   Enter apply + close   Esc close (keep result)\n\
           Waveform below updates live — red = detected, green/yellow = known.\n"
            .to_string(),
    );

    let content = Paragraph::new(lines.join("")).block(
        Block::new()
            .borders(Borders::ALL)
            .title(" Peak detection parameters ")
            .border_style(Color::Yellow),
    );
    frame.render_widget(content, area);
}
