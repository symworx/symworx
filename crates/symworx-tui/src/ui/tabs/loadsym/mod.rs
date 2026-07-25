use ratatui::{
    layout::Rect,
    style::Color,
    widgets::{
        Block,
        Borders,
        Padding,
        Paragraph,
    },
    Frame,
};

use crate::app::{
    App,
    LoadSymView,
};

mod calendar;
mod list;
mod metrics;
mod optimization;
mod util;
mod workout;

pub fn render_loadsym_tab(frame: &mut Frame, app: &App, area: Rect) {
    if app.help_mode {
        let body = match app.loadsym_view {
            LoadSymView::List => {
                "LoadSym — home\n\
                 Close help:  Esc  or  Alt-?\n\n\
                 \n\
                 VIEWS\n\n\
                   ↑ ↓  or  1–4           select\n\
                   Enter                  open selected view\n\
                   Ctrl+← / Ctrl+→        cycle strip views\n\n\
                   1  Workout Analysis    single ride · charts · SEPi/TSLi\n\
                   2  Metrics / Library   per-ride table · trends · bi-plots\n\
                   3  Calendar            daily/weekly load · catalog\n\
                   4  Optimization        multi-day plan · form/fatigue\n\n\
                 \n\
                 SHORTCUTS ON THIS LIST\n\n\
                   o                      open activity file browser\n\
                   i                      load newest .fit/CSV\n\
                   r                      reload SQLite catalog\n\
                   g                      synthetic demo daily loads\n\n\
                 Archive: $VELOFIT_HOME (default ~/velofit).\n\
                 Catalog is personal (never in git).\n"
            }
            LoadSymView::Workout => {
                "LoadSym — Workout Analysis\n\
                 Close help:  Esc  or  Alt-?\n\n\
                 \n\
                 OPEN A RIDE\n\n\
                   o                      file browser (↑↓ Enter)\n\
                   i / a                  newest file under archive dirs\n\
                   From Calendar          n/p ride · Enter/o open here\n\
                   r                      clear loaded activity\n\n\
                 Panel layout is kept when reloading (i/o) until you clear.\n\n\
                 \n\
                 CHARTS  (line, BioSym-style)\n\n\
                   1  power (W)           toggle show/hide\n\
                   2  heart rate          remaining panels share height\n\
                   3  speed (km/h)\n\
                   4  cadence (rpm)\n\
                   5  elevation (m)\n\
                   ← →                    pan shared time window\n\
                   ● open  ○ closed  ∅ no data in file\n\n\
                 \n\
                 METRICS\n\n\
                   f / F                  FTP ±5 W (SEPi / TSLi)\n\
                   t / T                  threshold ±\n\
                   d / D                  min duration ±\n\
                   Esc                    back to LoadSym list\n"
            }
            LoadSymView::Calendar => {
                "LoadSym — Calendar\n\
                 Close help:  Esc  or  Alt-?\n\n\
                 \n\
                 NAVIGATION\n\n\
                   ↑ ↓  /  k j            day (newest first on screen)\n\
                   ← →  /  h l            week aggregate\n\
                   Home / End             first / last day\n\
                   PgUp / PgDn            jump 10 days\n\
                   .                      jump to most recent day\n\n\
                 \n\
                 RIDES ON FOCUSED DAY\n\n\
                   n / p                  next / previous file\n\
                   Enter  /  o            open in Workout Analysis\n\n\
                 \n\
                 DATA\n\n\
                   r                      reload catalog ($VELOFIT_HOME/db)\n\
                   g                      demo daily series\n\
                   Esc                    back to LoadSym list\n\n\
                 Metrics: TSLi, ACLi, monotony, strain (LOADsym names).\n"
            }
            LoadSymView::Optimization => {
                "LoadSym — Programming Optimization\n\
                 Close help:  Esc  or  Alt-?\n\n\
                 \n\
                 GOAL  (default from form / fatigue / ACLi)\n\n\
                   auto                   scored on enter / catalog reload\n\
                   1  Recovery            ~20–55% of chronic load\n\
                   2  Maintenance         ~85–115%C, modulated days\n\
                   3  Overload            ~115–140%C\n\
                   1/2/3                  override; sticks until re-enter\n\n\
                 \n\
                 PLAN\n\n\
                   − / +                  horizon days (2…10)\n\
                   Enter                  recompute plan\n\
                   r                      reload catalog (+ re-suggest if no override)\n\
                   g                      28-day demo loads + replan\n\
                   Esc                    back to LoadSym list\n\n\
                 Charts: recent load + readiness (history | projection).\n\
                 Success = chronic load band; ACLi is advisory only.\n"
            }
            LoadSymView::Metrics => {
                "LoadSym — Metrics / Library\n\
                 Close help:  Esc  or  Alt-?\n\n\
                 \n\
                 TABLE\n\n\
                   ↑ ↓                  select ride (newest first)\n\
                   PgUp / PgDn          jump 10\n\
                   Home / End           first / last\n\
                   Enter / o            open in Workout Analysis\n\
                   r                    reload catalog\n\n\
                 \n\
                 CHARTS  (below table)\n\n\
                   v                    toggle trend ↔ bi-plot\n\n\
                 Trend  (metric vs ride order):\n\
                   1–8                  pick Y field\n\
                     1 TSLi  2 SEPi  3 avgW  4 dur  5 avgHR\n\
                     6 SRIi  7 work  8 maxW\n\n\
                 Bi-plot  (X vs Y):\n\
                   x / X                cycle X axis\n\
                   y / Y                cycle Y axis\n\
                   1–8                  set Y quickly (same map)\n\n\
                 Focused table row is highlighted on the chart.\n\
                 Esc                  back to LoadSym list\n"
            }
        };
        let global = "\n\
             GLOBAL\n\n\
               Ctrl+← / Ctrl+→     Workout · Metrics · Calendar · Optimization\n\
               Ctrl+H              Home\n\
               Esc Esc / Ctrl+Q    quit (Esc-Esc at roots only)\n\
               Alt-?               help\n";
        let help = Paragraph::new(format!("{body}{global}")).block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(match app.loadsym_view {
                    LoadSymView::List => " Help — LoadSym ",
                    LoadSymView::Workout => " Help — LoadSym · Workout ",
                    LoadSymView::Calendar => " Help — LoadSym · Calendar ",
                    LoadSymView::Optimization => " Help — LoadSym · Optimization ",
                    LoadSymView::Metrics => " Help — LoadSym · Metrics ",
                }),
        );
        frame.render_widget(help, area);
        return;
    }

    let outer = Block::new()
        .title(" LoadSym — Training Load, ACLi, Optimization ")
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1))
        .border_style(Color::Yellow);

    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    // File-open modal overlays Workout (and List→open) content.
    if app.pending_workout_open {
        list::render_workout_open_modal(frame, app, inner);
        return;
    }

    match app.loadsym_view {
        LoadSymView::List => {
            list::render_loadsym_list(frame, app, inner);
        }
        LoadSymView::Workout => {
            workout::render_workout_view(frame, app, inner);
        }
        LoadSymView::Calendar => {
            calendar::render_calendar_view(frame, app, inner);
        }
        LoadSymView::Optimization => {
            optimization::render_optimization_view(frame, app, inner);
        }
        LoadSymView::Metrics => {
            metrics::render_metrics_view(frame, app, inner);
        }
    }
}
