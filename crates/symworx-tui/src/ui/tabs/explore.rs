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

use crate::app::App;

/// Visible sample window width for the Explore chart (x-axis pan step uses the same value).
pub const EXPLORE_VIEW_LEN: usize = 300;

pub fn render_explore_tab(frame: &mut Frame, app: &App, area: Rect) {
    if app.help_mode {
        let help = Paragraph::new(
            "Explore help (M-? or Esc to close)\n\n\
             • Stats + Chart for loaded BioSym signal (PPG / respiration / stride)\n\
             • Multiple waveforms: use MultiWaveformDemo preset (generates v1,v2.. variants)\n\
             • p : open process menu (MA / Median / Detrend)\n\
             • r : reset to original signal\n\
             • ← → / h l : pan x-axis viewport (window of ~300 samples)\n\
             • Esc : back to Import tab\n\
             After generate/load: use p to try filters, then Dynamics (Ctrl+3) for RQA.",
        )
        .block(
            Block::new()
                .borders(Borders::ALL)
                .title(" Help — Explore (BioSym) "),
        );
        frame.render_widget(help, area);
        return;
    }

    if app.pending_process {
        let names = ["Moving Average", "Median Filter", "Detrend (mean)"];
        let mut lines = vec![
            "\n\nSignal Processing\n\n".to_string(),
            "Choose operation:\n\n".to_string(),
        ];
        for (i, name) in names.iter().enumerate() {
            let sel = if app.process_selection == i { ">" } else { " " };
            lines.push(format!(
                "{} {} (window {})\n",
                sel, name, app.process_window
            ));
        }
        lines.push("\n↑↓ select   ←→/-+ adjust window   Enter apply   Esc cancel\n".to_string());
        let content = Paragraph::new(lines.join(""))
            .block(Block::new().borders(Borders::ALL).title(" Process "));
        frame.render_widget(content, area);
        return;
    }

    let block = Block::new()
        .title(" Explore — BioSym Waveforms ")
        .borders(Borders::ALL)
        .border_style(Color::Magenta);

    if let Some(signal) = &app.loaded_signal {
        let stats = crate::app::compute_basic_stats(&signal.current);

        // Pannable x viewport: app.explore_scroll is the start sample index.
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

        let y_min = stats.min;
        let y_max = stats.max;

        // Build points for Chart (x = sample index in viewport, y = value)
        let data: Vec<(f64, f64)> = visible
            .iter()
            .enumerate()
            .map(|(i, &v)| ((start + i) as f64, v))
            .collect();

        let datasets = vec![Dataset::default()
            .name("signal")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::LightCyan))
            .data(&data)];

        // X axis: sample index (pannable window)
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
        let lines = vec![
            format!(
                "File: {}  |  view {}..{} / {} samples{}",
                signal.name, start, end, n, pan_hint
            ),
            format!(
                "Mean: {:.2}  Std: {:.2}  Min: {:.2}  Max: {:.2}  Median: {:.2}",
                stats.mean, stats.std, stats.min, stats.max, stats.median
            ),
        ];

        let content = Paragraph::new(lines.join("\n")).block(block);
        let chunks = Layout::vertical([Constraint::Length(4), Constraint::Min(1)]).split(area);
        frame.render_widget(content, chunks[0]);
        frame.render_widget(chart, chunks[1]);
    } else {
        let content = Paragraph::new(
            "Load or Generate BioSym signal (Import or generate presets)\n\n\
             Presets support multiple waveform variants (PPG v1/v2, respiration, stride).\n\
             Chart: x/y axes; ←→ pan long traces.\n\n(Expanded via symworx-signal + full multi-overlay planned)",
        )
        .block(block);
        frame.render_widget(content, area);
    }
}
