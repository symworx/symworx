use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    symbols,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
};
use crate::app::App;

pub fn render_explore_tab(frame: &mut Frame, app: &App, area: Rect) {
    if app.help_mode {
        let help = Paragraph::new(
            "Explore help (M-? or Esc to close)\n\n\
             • Stats + Chart for loaded BioSym signal (PPG / respiration / stride)\n\
             • Multiple waveforms: use MultiWaveformDemo preset (generates v1,v2.. variants)\n\
             • p : open process menu (MA / Median / Detrend) — y-axis carries over fixed range\n\
             • r : reset to original signal\n\
             • For long series: rolling x viewport (recent window); full panning planned\n\
             After generate/load: use p to try filters, then Dynamics (Ctrl+3) for RQA.",
        )
        .block(Block::new().borders(Borders::ALL).title(" Help — Explore (BioSym) "));
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
        lines.push(
            "\n↑↓ select   ←→/-+ adjust window   Enter apply   Esc cancel\n".to_string(),
        );
        let content = Paragraph::new(lines.join(""))
            .block(Block::new().borders(Borders::ALL).title(" Process (y fixed after apply) "));
        frame.render_widget(content, area);
        return;
    }

    let block = Block::new()
        .title(" Explore — BioSym Waveforms (x/y axes + fixed y + rolling x) ")
        .borders(Borders::ALL)
        .border_style(Color::Magenta);

    if let Some(signal) = &app.loaded_signal {
        let stats = crate::app::compute_basic_stats(&signal.current);

        // Rolling x viewport for longer time series (show recent window, 'rolls' to end)
        let view_len: usize = 300; // visible samples; adjust for perf/detail
        let n = signal.current.len();
        let scroll = 0usize; // TODO: wire to app.explore_scroll + input handling for panning
        let start = if n > view_len {
            n.saturating_sub(view_len) // rolling to recent end
        } else {
            0
        };
        let end = n;
        let visible: Vec<f64> = if n > 0 {
            signal.current[start..end].to_vec()
        } else {
            vec![]
        };

        // Fixed y axis carry-over: use current stats min/max (stable across processing until reset/load new)
        // For true original-range lock, store y_bounds in LoadedSignal at import time.
        let y_min = stats.min;
        let y_max = stats.max;
        let y_range = if y_max > y_min { y_max - y_min } else { 1.0 };

        // Build points for Chart (x = sample index in viewport, y = value)
        let data: Vec<(f64, f64)> = visible
            .iter()
            .enumerate()
            .map(|(i, &v)| ( (start + i) as f64 , v ))
            .collect();

        let datasets = vec![Dataset::default()
            .name("signal")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::LightCyan))
            .data(&data)];

        // X axis: sample index (rolling window)
        let x_axis = Axis::default()
            .title("Sample index (rolling window)")
            .style(Style::default().fg(Color::Gray))
            .bounds([start as f64, end as f64])
            .labels(vec![
                format!("{}", start).into(),
                format!("{}", (start + end) / 2).into(),
                format!("{}", end).into(),
            ]);

        // Y axis: amplitude, fixed carry-over
        let y_axis = Axis::default()
            .title("Amplitude (fixed y)")
            .style(Style::default().fg(Color::Gray))
            .bounds([y_min, y_max])
            .labels(vec![
                format!("{:.2}", y_min).into(),
                format!("{:.2}", (y_min + y_max) / 2.0).into(),
                format!("{:.2}", y_max).into(),
            ]);

        let chart = Chart::new(datasets)
            .block(Block::new())
            .x_axis(x_axis)
            .y_axis(y_axis);

        let lines = vec![
            format!(
                "File: {}  |  view {}..{} / {} samples (rolling x)  |  variants via MultiWaveformDemo",
                signal.name, start, end, n
            ),
            format!(
                "Mean: {:.2}  Std: {:.2}  Min: {:.2}  Max: {:.2}  Median: {:.2} (y fixed)",
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
             Presets now support multiple waveform variants (PPG v1/v2, respiration, stride).\n\
             Use Chart viz with x/y axes, fixed y after processing, rolling x for long traces.\n\n(Expanded via symworx-signal + full multi-overlay planned)",
        )
        .block(block);
        frame.render_widget(content, area);
    }
}
