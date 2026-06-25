use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::Span,
    widgets::{Block, Borders, Paragraph, Tabs},
};
use crate::app::{App, Tab};

pub mod tabs;

pub fn ui(frame: &mut Frame, app: &mut App) {
    let main_layout = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(8),
        Constraint::Length(2),
    ])
    .split(frame.area());

    let tab_titles: Vec<Span> = crate::app::tab_titles();

    let tabs = Tabs::new(tab_titles)
        .block(Block::new().borders(Borders::BOTTOM))
        .select(app.current_tab.index())
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, main_layout[0]);

    let action_bar = render_action_bar(app);
    frame.render_widget(action_bar, main_layout[1]);

    match app.current_tab {
        Tab::Import => tabs::import::render_import_tab(frame, app, main_layout[2]),
        Tab::Explore => tabs::explore::render_explore_tab(frame, app, main_layout[2]),
        Tab::Dynamics => tabs::dynamics::render_dynamics_tab(frame, app, main_layout[2]),
        Tab::Spatial => tabs::spatial::render_spatial_tab(frame, app, main_layout[2]),
    }

    let footer = Paragraph::new(format!(" {}  •  q: Quit", app.status))
        .centered()
        .dim()
        .block(Block::new().borders(Borders::TOP));
    frame.render_widget(footer, main_layout[3]);
}

pub fn render_action_bar(app: &App) -> Paragraph<'_> {
    let (text, style) = match app.current_tab {
        Tab::Import => {
            if app.pending_generate {
                (
                    "  [1] PPG   [2] Respiration   [3] Stride intervals   [Esc] Cancel",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )
            } else if app.filter_mode {
                (
                    "  Filtering...  [Esc] or [Enter] to exit filter",
                    Style::default().fg(Color::Cyan),
                )
            } else {
                (
                    "  [/] Filter   [Ctrl+G] Generate demo   [c] Convert   [Enter] Load   [↑↓] Navigate",
                    Style::default().fg(Color::DarkGray),
                )
            }
        }
        Tab::Explore => (
            "  [p] Process (MA / Median / Detrend)   [r] Reset to original   Stats + Sparkline active",
            Style::default().fg(Color::DarkGray),
        ),
        Tab::Dynamics => (
            "  [Coming soon: RQA, Recurrence Plots, Nonlinear Analysis]",
            Style::default().fg(Color::DarkGray),
        ),
        Tab::Spatial => (
            "  [←→] frame   [g] regen   [i] infer   [1-9] jump event   [l] legend   M-? help",
            Style::default().fg(Color::DarkGray),
        ),
    };
    Paragraph::new(text).style(style)
}
