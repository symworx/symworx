use ratatui::{
    layout::{
        Alignment,
        Constraint,
        Layout,
        Rect,
    },
    style::{
        Color,
        Modifier,
        Style,
        Stylize,
    },
    text::{
        Line,
        Span,
    },
    widgets::{
        Block,
        Borders,
        Paragraph,
    },
    Frame,
};

use crate::app::{
    App,
    Tab,
};

pub mod tabs;

/// Inset a rect by `n` columns on left/right (chrome / content side gutters).
fn h_inset(area: Rect, n: u16) -> Rect {
    if area.width <= n.saturating_mul(2) {
        return area;
    }
    Rect {
        x: area.x.saturating_add(n),
        y: area.y,
        width: area.width.saturating_sub(n.saturating_mul(2)),
        height: area.height,
    }
}

fn is_home(app: &App) -> bool {
    app.current_workflow == crate::app::Workflow::Home || app.current_tab == Tab::Home
}

pub fn ui(frame: &mut Frame, app: &mut App) {
    let main_layout = Layout::vertical([
        Constraint::Length(1), // header
        Constraint::Length(1), // action / status
        Constraint::Min(8),    // content
        Constraint::Length(3), // footer: rule + nav + air
    ])
    .split(frame.area());

    // Top chrome (side gutters so text is not flush to terminal edges):
    //   Left: "SymView"
    //   Center: module name
    //   Right: current sub-tab / HOME
    let header_area = h_inset(main_layout[0], 1);
    let header_chunks =
        Layout::horizontal([Constraint::Length(9), Constraint::Fill(1), Constraint::Min(12)]).split(header_area);

    let left = Paragraph::new("SymView").style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    frame.render_widget(left, header_chunks[0]);

    let parent = match app.current_workflow {
        crate::app::Workflow::Home => "",
        crate::app::Workflow::BioSym => "BioSym",
        crate::app::Workflow::SpatialSym => "SpatialSym",
        crate::app::Workflow::LoadSym => "LoadSym",
        crate::app::Workflow::StatsSym => "StatsSym",
    };
    let center = Paragraph::new(parent)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(center, header_chunks[1]);

    let right = if is_home(app) {
        "HOME".to_string()
    } else {
        match app.current_workflow {
            crate::app::Workflow::BioSym => app.current_tab.title().to_string(),
            crate::app::Workflow::StatsSym => app.stats_view.title().to_string(),
            crate::app::Workflow::SpatialSym => "Spatial".to_string(),
            crate::app::Workflow::LoadSym => match app.loadsym_view {
                crate::app::LoadSymView::List => "Home".to_string(),
                crate::app::LoadSymView::Workout => "Workout".to_string(),
                crate::app::LoadSymView::Metrics => "Metrics".to_string(),
                crate::app::LoadSymView::Calendar => "Calendar".to_string(),
                crate::app::LoadSymView::Optimization => "Optimization".to_string(),
            },
            _ => app.current_tab.title().to_string(),
        }
    };
    let right_p = Paragraph::new(right)
        .alignment(Alignment::Right)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    frame.render_widget(right_p, header_chunks[2]);

    // Action bar: status + hints (also inset from terminal edges).
    let action_bar = render_action_bar(app);
    frame.render_widget(action_bar, h_inset(main_layout[1], 1));

    // Main content: slight side gutters so boxes aren't flush to the frame edge.
    let content = h_inset(main_layout[2], 1);
    if is_home(app) {
        tabs::home::render_home_tab(frame, app, content);
    } else if app.current_workflow == crate::app::Workflow::LoadSym || app.current_tab == Tab::LoadSym {
        tabs::loadsym::render_loadsym_tab(frame, app, content);
    } else if app.current_workflow == crate::app::Workflow::StatsSym || app.current_tab == Tab::Stats {
        tabs::stats::render_stats_tab(frame, app, content);
    } else {
        match app.current_tab {
            Tab::Import => tabs::import::render_import_tab(frame, app, content),
            Tab::Explore => tabs::explore::render_explore_tab(frame, app, content),
            Tab::Dynamics => tabs::dynamics::render_dynamics_tab(frame, app, content),
            Tab::Generate => tabs::bio_generate::render_bio_generate_tab(frame, app, content),
            Tab::Spatial => tabs::spatial::render_spatial_tab(frame, app, content),
            Tab::LoadSym => tabs::loadsym::render_loadsym_tab(frame, app, content),
            Tab::Stats => tabs::stats::render_stats_tab(frame, app, content),
            Tab::Home => tabs::home::render_home_tab(frame, app, content),
        }
    }

    // Footer chrome:
    //   row 0 — top rule (air above content)
    //   row 1 — Home | module tabs | quit  (side gutters + column padding)
    //   row 2 — empty buffer under the nav line
    let footer_area = main_layout[3];
    let footer_rows =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)]).split(footer_area);

    frame.render_widget(Block::new().borders(Borders::TOP), footer_rows[0]);

    let nav = h_inset(footer_rows[1], 1);
    let footer_chunks = Layout::horizontal([
        Constraint::Length(15), // Ctrl+H Home
        Constraint::Fill(1),    // module tabs (centered)
        Constraint::Length(18), // quit
    ])
    .split(nav);

    let home_style = if is_home(app) {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    // Leading/trailing spaces keep labels off the column edges.
    let home_p = Paragraph::new(Span::styled(" Ctrl+H Home ", home_style));

    let tabs_p = Paragraph::new(module_tab_line(app)).alignment(Alignment::Center);

    let right_quit = Paragraph::new(if app.esc_quit_pending {
        " Esc again quit "
    } else {
        " Esc Esc · Ctrl+Q "
    })
    .alignment(Alignment::Right)
    .dim();

    frame.render_widget(home_p, footer_chunks[0]);
    frame.render_widget(tabs_p, footer_chunks[1]);
    frame.render_widget(right_quit, footer_chunks[2]);
}

/// Module sub-tabs L→R, centered in the footer; **bold cyan** = current.
fn module_tab_line(app: &App) -> Line<'static> {
    let labels: Vec<(&'static str, bool)> = match app.current_workflow {
        crate::app::Workflow::Home => {
            // On Home, show available modules (selection not a sub-tab).
            return Line::from(Span::styled(
                "  1 BioSym  ·  2 StatsSym  ·  3 LoadSym  ·  4 Spatial  ",
                Style::default().fg(Color::DarkGray),
            ));
        }
        crate::app::Workflow::BioSym => Tab::BIOSYM_TABS
            .iter()
            .map(|&t| (t.title(), app.current_tab == t))
            .collect(),
        crate::app::Workflow::StatsSym => vec![
            ("Import", app.stats_view == crate::app::StatsView::Import),
            ("Lab", app.stats_view == crate::app::StatsView::Lab),
            ("Generate", app.stats_view == crate::app::StatsView::Generate),
        ],
        crate::app::Workflow::LoadSym => {
            use crate::app::LoadSymView;
            vec![
                ("Workout", app.loadsym_view == LoadSymView::Workout),
                ("Metrics", app.loadsym_view == LoadSymView::Metrics),
                ("Calendar", app.loadsym_view == LoadSymView::Calendar),
                ("Optimization", app.loadsym_view == LoadSymView::Optimization),
            ]
        }
        crate::app::Workflow::SpatialSym => vec![("Spatial", true)],
    };

    let mut spans: Vec<Span<'static>> = Vec::new();
    // Soft horizontal pad so the strip isn't tight against Home / quit columns.
    spans.push(Span::raw(" "));
    for (i, (label, selected)) in labels.into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ·  ", Style::default().fg(Color::DarkGray)));
        }
        if selected {
            spans.push(Span::styled(
                format!(" {label} "),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(format!(" {label} "), Style::default().fg(Color::DarkGray)));
        }
    }
    spans.push(Span::raw(" "));
    Line::from(spans)
}

pub fn render_action_bar(app: &App) -> Paragraph<'_> {
    // Status + contextual Esc / key hints. Home / tabs / quit live in the footer.

    if app.pending_delete.is_some() {
        return Paragraph::new("  Delete file?   [y] confirm   [n] / [Esc] cancel")
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    }
    if app.filter_mode {
        return Paragraph::new("  Filtering…  [Esc] or [Enter] to exit filter").style(Style::default().fg(Color::Cyan));
    }

    let esc = if app.pending_process {
        "[Esc] cancel process  ·  "
    } else if app.pending_peak_params {
        "[Esc] close peak params  ·  "
    } else if app.pending_rqa {
        "[Esc] cancel RQA  ·  "
    } else if app.pending_spatial_import {
        "[Esc] cancel import  ·  "
    } else if app.is_live() {
        "[Esc] stop live  ·  "
    } else if app.current_workflow == crate::app::Workflow::LoadSym {
        if app.pending_workout_open {
            "[Esc] cancel open  ·  "
        } else {
            match app.loadsym_view {
                crate::app::LoadSymView::List => "",
                _ => "[Esc] back to list  ·  ",
            }
        }
    } else if app.current_workflow == crate::app::Workflow::StatsSym {
        match app.stats_view {
            crate::app::StatsView::Import => "",
            _ => "[Esc] Import  ·  ",
        }
    } else if app.current_workflow == crate::app::Workflow::BioSym
        && (app.current_tab == Tab::Explore || app.current_tab == Tab::Generate)
    {
        "[Esc] Import  ·  "
    } else if app.current_workflow == crate::app::Workflow::BioSym && app.current_tab == Tab::Dynamics {
        "[Esc] Explore  ·  "
    } else {
        ""
    };

    let hints = if app.is_live() {
        "Ctrl+L restart  ·  Alt-? help"
    } else if app.current_workflow == crate::app::Workflow::BioSym {
        "Ctrl+←→ tabs  ·  Ctrl+G generate  ·  Ctrl+L live  ·  Alt-? help"
    } else if app.current_workflow == crate::app::Workflow::StatsSym {
        "Ctrl+←→ tabs  ·  Ctrl+G generate  ·  Alt-? help"
    } else if app.current_workflow == crate::app::Workflow::Home {
        "↑↓ select  ·  Enter open  ·  Alt-? help"
    } else {
        "↑↓  ·  Enter  ·  Alt-? help"
    };

    // Leading status (truncated-friendly); esc + hints after.
    let text = format!(" {}  ·  {}{}", app.status, esc, hints);
    Paragraph::new(text).style(Style::default().fg(Color::DarkGray))
}
