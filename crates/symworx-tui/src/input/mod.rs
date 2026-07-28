// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Keyboard input: global chords + dispatch into per-workflow handlers.

mod biosym;
mod common;
mod home;
mod loadsym;
mod spatial;
mod stats;

use crossterm::event::{
    KeyCode,
    KeyModifiers,
};

use crate::app::{
    App,
    Tab,
};

pub fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    // Hard quit: Ctrl+Q always. Esc-Esc at root screens (see esc_root_or_quit).
    // Bare `q` is not quit — collides with typing.
    if matches!(code, KeyCode::Char('q') | KeyCode::Char('Q')) && modifiers.contains(KeyModifiers::CONTROL) {
        return true;
    }

    // Any non-Esc key disarms double-Esc quit.
    if code != KeyCode::Esc {
        app.clear_esc_quit();
    }

    if code == KeyCode::Char('?') && modifiers.contains(KeyModifiers::ALT) {
        app.help_mode = !app.help_mode;
        app.clear_esc_quit();
        return false;
    }

    // Refresh must be reliable (even in submodes / while typing) — early return per conventions
    if (code == KeyCode::Char('r') && modifiers.contains(KeyModifiers::CONTROL)) || code == KeyCode::F(5) {
        app.refresh_file_list();
        app.status = "Refreshed file list (Ctrl+R / F5)".to_string();
        app.ensure_status_for_current_tab();
        return false;
    }

    if app.help_mode {
        if code == KeyCode::Esc {
            app.help_mode = false;
            app.clear_esc_quit();
            return false;
        }
        // Still allow global home navigation from help dashboard
        if (code == KeyCode::Char('h') || code == KeyCode::Char('H')) && modifiers.contains(KeyModifiers::CONTROL) {
            app.help_mode = false;
            app.clear_submodes();
            app.switch_workflow(crate::app::Workflow::Home);
            return false;
        }
        if code == KeyCode::Char('0') && !modifiers.contains(KeyModifiers::CONTROL) {
            app.help_mode = false;
            if app.current_workflow != crate::app::Workflow::Home {
                app.clear_submodes();
                app.switch_workflow(crate::app::Workflow::Home);
            }
            return false;
        }
        // allow re-toggle help
        if code == KeyCode::Char('?') && modifiers.contains(KeyModifiers::ALT) {
            app.help_mode = false;
            return false;
        }
        return false;
    }

    // Home / workflow selector access (always available, early)
    if (code == KeyCode::Char('h') || code == KeyCode::Char('H')) && modifiers.contains(KeyModifiers::CONTROL) {
        app.clear_submodes();
        app.switch_workflow(crate::app::Workflow::Home);
        return false;
    }
    // Allow quick home from number 0 when appropriate (or always map)
    if code == KeyCode::Char('0') && !modifiers.contains(KeyModifiers::CONTROL) {
        if app.current_workflow != crate::app::Workflow::Home {
            app.clear_submodes();
            app.switch_workflow(crate::app::Workflow::Home);
        }
        return false;
    }

    match code {
        KeyCode::Char('1') if modifiers.contains(KeyModifiers::CONTROL) => {
            match app.current_workflow {
                crate::app::Workflow::StatsSym => {
                    app.stats_view = crate::app::StatsView::Import;
                    app.status = "StatsSym Import — ↑↓ files  Enter load  / filter  Ctrl+G generate".into();
                }
                _ => {
                    app.current_tab = Tab::Import;
                    app.current_workflow = crate::app::Workflow::BioSym;
                    app.ensure_status_for_current_tab();
                }
            }
            return false;
        }
        KeyCode::Char('2') if modifiers.contains(KeyModifiers::CONTROL) => {
            match app.current_workflow {
                crate::app::Workflow::StatsSym => {
                    app.stats_view = crate::app::StatsView::Lab;
                    app.status = "StatsSym Lab — 1–4 task  x/y cols  Enter run  h residual".into();
                }
                _ => {
                    app.current_tab = Tab::Explore;
                    app.current_workflow = crate::app::Workflow::BioSym;
                    app.ensure_status_for_current_tab();
                }
            }
            return false;
        }
        KeyCode::Char('3') if modifiers.contains(KeyModifiers::CONTROL) => {
            match app.current_workflow {
                crate::app::Workflow::StatsSym => {
                    app.stats_view = crate::app::StatsView::Generate;
                    app.status = "StatsSym Generate — ↑↓ preset  n/N size  s/S seed  +/− noise  Enter".into();
                }
                _ => {
                    app.current_tab = Tab::Dynamics;
                    app.current_workflow = crate::app::Workflow::BioSym;
                    app.ensure_status_for_current_tab();
                }
            }
            return false;
        }
        KeyCode::Char('4') if modifiers.contains(KeyModifiers::CONTROL) => {
            // Note: from BioSym we prefer not mixing, but quick switch is allowed via Ctrl+4
            app.current_tab = Tab::Spatial;
            app.current_workflow = crate::app::Workflow::SpatialSym;
            app.ensure_status_for_current_tab();
            return false;
        }
        KeyCode::Left if modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_workflow == crate::app::Workflow::Home {
                return false;
            }
            // Scope navigation to parent workflow's subtabs.
            match app.current_workflow {
                crate::app::Workflow::BioSym => {
                    app.pending_generate = false;
                    app.current_tab = if Tab::BIOSYM_TABS.contains(&app.current_tab) {
                        app.current_tab.biosym_prev()
                    } else {
                        Tab::Import
                    };
                    app.ensure_status_for_current_tab();
                }
                crate::app::Workflow::StatsSym => {
                    app.stats_view = app.stats_view.prev();
                    app.status = format!("StatsSym {}  ·  Ctrl+←→ tabs  ·  Ctrl+H Home", app.stats_view.title());
                }
                crate::app::Workflow::LoadSym => {
                    // Cancel open modal so strip navigation is never trapped.
                    app.pending_workout_open = false;
                    let next = app.loadsym_view.strip_prev();
                    loadsym::apply_loadsym_strip_view(app, next);
                }
                crate::app::Workflow::SpatialSym => {
                    // Single Spatial strip item today — no-op (reserved).
                }
                crate::app::Workflow::Home => {}
            }
            return false;
        }
        KeyCode::Right if modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_workflow == crate::app::Workflow::Home {
                return false;
            }
            match app.current_workflow {
                crate::app::Workflow::BioSym => {
                    app.pending_generate = false;
                    app.current_tab = if Tab::BIOSYM_TABS.contains(&app.current_tab) {
                        app.current_tab.biosym_next()
                    } else {
                        Tab::Import
                    };
                    app.ensure_status_for_current_tab();
                }
                crate::app::Workflow::StatsSym => {
                    app.stats_view = app.stats_view.next();
                    app.status = format!("StatsSym {}  ·  Ctrl+←→ tabs  ·  Ctrl+H Home", app.stats_view.title());
                }
                crate::app::Workflow::LoadSym => {
                    app.pending_workout_open = false;
                    let next = app.loadsym_view.strip_next();
                    loadsym::apply_loadsym_strip_view(app, next);
                }
                crate::app::Workflow::SpatialSym => {}
                crate::app::Workflow::Home => {}
            }
            return false;
        }
        KeyCode::Char('g') | KeyCode::Char('G') if modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_workflow == crate::app::Workflow::StatsSym {
                // StatsSym: open Generate tab (full preset UI — n/seed/noise).
                app.stats_view = crate::app::StatsView::Generate;
                app.manual_path.clear();
                app.file_filter.clear();
                app.filter_mode = false;
                app.status = "StatsSym Generate — ↑↓ preset  n/N  s/S seed  +/− noise  Enter → Lab".into();
            } else {
                // BioSym: dedicated Generate tab (parity with StatsSym).
                app.current_workflow = crate::app::Workflow::BioSym;
                app.current_tab = Tab::Generate;
                app.pending_generate = false;
                app.manual_path.clear();
                app.file_filter.clear();
                app.filter_mode = false;
                app.status = "BioSym Generate — ↑↓ preset  Enter generate → Explore  ·  1/2/3 quick".into();
            }
            return false;
        }
        // Live stream (simulator) — BioSym only. Bare `l`/`L` is Explore pan.
        KeyCode::Char('l') | KeyCode::Char('L') if modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_workflow == crate::app::Workflow::BioSym {
                app.start_live_simulator();
            } else {
                app.status =
                    "Ctrl+L live simulator is BioSym only — Home → 1 BioSym (or Explore), then Ctrl+L".to_string();
            }
            return false;
        }
        _ => {}
    }

    // Route Home first (landing selector takes precedence for its keys)
    if app.current_workflow == crate::app::Workflow::Home || app.current_tab == Tab::Home {
        return home::handle_home_keys(app, code, modifiers);
    }

    match app.current_tab {
        Tab::Import => biosym::handle_import_keys(app, code, modifiers),
        Tab::Explore => biosym::handle_explore_keys(app, code, modifiers),
        Tab::Dynamics => biosym::handle_dynamics_keys(app, code),
        Tab::Generate => biosym::handle_bio_generate_keys(app, code),
        Tab::Spatial => spatial::handle_spatial_keys(app, code, modifiers),
        Tab::LoadSym => loadsym::handle_loadsym_keys(app, code, modifiers),
        Tab::Stats => stats::handle_stats_keys(app, code, modifiers),
        Tab::Home => home::handle_home_keys(app, code, modifiers),
    }
}
