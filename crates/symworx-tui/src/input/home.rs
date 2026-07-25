use crossterm::event::{
    KeyCode,
    KeyModifiers,
};

use crate::app::App;

pub(crate) fn handle_home_keys(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
    // Arrow and digit selection for paths on landing
    match code {
        KeyCode::Up => {
            if app.home_selection > 0 {
                app.home_selection -= 1;
            }
            return false;
        }
        KeyCode::Down => {
            if app.home_selection < 3 {
                app.home_selection += 1;
            }
            return false;
        }
        KeyCode::Char('1') => {
            app.switch_workflow(crate::app::Workflow::BioSym);
            return false;
        }
        KeyCode::Char('2') => {
            app.switch_workflow(crate::app::Workflow::StatsSym);
            return false;
        }
        KeyCode::Char('3') => {
            app.switch_workflow(crate::app::Workflow::LoadSym);
            return false;
        }
        KeyCode::Char('4') => {
            app.switch_workflow(crate::app::Workflow::SpatialSym);
            return false;
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if app.home_selection == 2 {
                app.status =
                    "LoadSym: 1 Workout  2 Metrics  3 Calendar  4 Optimization".to_string();
            }
            return false;
        }
        KeyCode::Char('i') | KeyCode::Char('I') => {
            if app.home_selection == 2 {
                app.status =
                    "LoadSym: import via activity files (FIT via symworx-io) — see LoadSym tab."
                        .to_string();
            } else if app.home_selection == 3 {
                app.switch_workflow(crate::app::Workflow::SpatialSym);
            }
            return false;
        }
        KeyCode::Enter => {
            match app.home_selection {
                0 => app.switch_workflow(crate::app::Workflow::BioSym),
                1 => app.switch_workflow(crate::app::Workflow::StatsSym),
                2 => app.switch_workflow(crate::app::Workflow::LoadSym),
                3 => app.switch_workflow(crate::app::Workflow::SpatialSym),
                _ => {}
            }
            return false;
        }
        KeyCode::Esc => {
            // Root: Esc-Esc quits (Ctrl+Q also works).
            return app.esc_root_or_quit();
        }
        _ => {}
    }
    false
}
