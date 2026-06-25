use ratatui::{
    Frame,
    layout::Rect,
    style::Color,
    widgets::{Block, Borders, Paragraph},
};
use crate::app::App;

pub fn render_dynamics_tab(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::new()
        .title(" Nonlinear Dynamics (RQA, Recurrence Plots, etc.) ")
        .borders(Borders::ALL)
        .border_style(Color::Green);

    let content = if app.loaded_signal.is_some() {
        Paragraph::new(
            "\n\nThis tab will host RQA, RecurrencePlot visualization,\n\
             embedding dimension analysis, and related nonlinear tools.\n\n\
             (Implementation planned after Explore tab is solid)",
        )
        .centered()
    } else {
        Paragraph::new("\n\nLoad a signal in the Import tab first.").centered()
    };

    frame.render_widget(content.block(block), area);
}
