use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Sparkline},
};
use crate::app::App;

pub fn render_explore_tab(frame: &mut Frame, app: &App, area: Rect) {
    if app.help_mode {
        let help = Paragraph::new(
            "Explore help (M-? or Esc to close)\n\n\
             • Stats + sparkline for loaded BioSym signal\n\
             • p : open process menu (MA / Median / Detrend)\n\
             • r : reset to original signal\n\n\
             After generate/load: use p to try filters, then Dynamics (Ctrl+3) for RQA."
        ).block(Block::new().borders(Borders::ALL).title(" Help — Explore "));
        frame.render_widget(help, area);
        return;
    }

    if app.pending_process {
        let names = ["Moving Average", "Median Filter", "Detrend (mean)"];
        let mut lines = vec!["\n\nSignal Processing\n\n".to_string(), "Choose operation:\n\n".to_string()];
        for (i, name) in names.iter().enumerate() {
            let sel = if app.process_selection == i { ">" } else { " " };
            lines.push(format!("{} {} (window {})\n", sel, name, app.process_window));
        }
        lines.push("\n←/→ or -/+ adjust window   Enter apply   Esc cancel\n".to_string());
        let content = Paragraph::new(lines.join("")).block(Block::new().borders(Borders::ALL).title(" Process "));
        frame.render_widget(content, area);
        return;
    }

    let block = Block::new()
        .title(" Explore (stats + sparkline) ")
        .borders(Borders::ALL)
        .border_style(Color::Magenta);

    if let Some(signal) = &app.loaded_signal {
        let stats = crate::app::compute_basic_stats(&signal.current);
        let mut lines = vec![
            format!("File: {}", signal.name),
            format!("Samples: {}", signal.n_samples),
            format!("Mean: {:.2}  Std: {:.2}  Min: {:.2}  Max: {:.2}  Median: {:.2}", stats.mean, stats.std, stats.min, stats.max, stats.median),
        ];

        let mut spark_data: Vec<u64> = vec![];
        if !signal.current.is_empty() {
            let min = stats.min;
            let max = stats.max;
            let range = if max > min { max - min } else { 1.0 };
            spark_data = signal.current.iter().map(|&v| (((v - min) / range) * 200.0) as u64).collect();
        }

        let spark = Sparkline::default()
            .block(Block::new())
            .data(&spark_data)
            .style(Style::default().fg(Color::LightCyan))
            .max(200);

        let content = Paragraph::new(lines.join("\n")).block(block);
        let chunks = Layout::vertical([Constraint::Length(5), Constraint::Min(1)]).split(area);
        frame.render_widget(content, chunks[0]);
        frame.render_widget(spark, chunks[1]);
    } else {
        let content = Paragraph::new("Load a signal in Import tab first.\n\n(Expanded filters available via symworx-signal in future updates; current: p for MA/Median/Detrend)").block(block);
        frame.render_widget(content, area);
    }
}
