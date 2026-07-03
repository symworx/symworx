use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};
use crate::app::App;

/// Empty template page for the LoadSym workflow.
pub fn render_loadsym_tab(frame: &mut Frame, app: &App, area: Rect) {
    if app.help_mode {
        let help = Paragraph::new(
            "LoadSym help (M-? or Esc to close)\n\n\
             Template for training load / ACWR / nutrition.\n\n\
             (No interactive sub-pages yet.)\n\
             Use Home (Ctrl+H) to switch paths.\n\
             Future: data import from BioSym signals + ride logs."
        ).block(Block::new().borders(Borders::ALL).title(" Help — LoadSym "));
        frame.render_widget(help, area);
        return;
    }

    let content = Paragraph::new(
        "\n\
         LoadSym\n\n\
         Training load metrics, ACWR, monotony, and nutrition analysis.\n\n\
         (Template page — implementation in progress.)\n\n\
         Future: integrate data from ride logs, BioSym signals, etc.\n"
    )
    .style(Style::default().fg(Color::DarkGray))
    .block(Block::new().borders(Borders::ALL).title(" LoadSym "));

    frame.render_widget(content, area);
}
