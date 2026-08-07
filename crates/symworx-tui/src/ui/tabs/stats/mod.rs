// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

use ratatui::{
    Frame,
    layout::Rect,
    style::Color,
    widgets::{
        Block,
        Borders,
        Padding,
        Paragraph,
    },
};

mod charts;
mod generate;
mod import;
mod lab;
mod placeholder;

use crate::app::{
    App,
    StatsView,
};

pub fn render_stats_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.help_mode {
        let body = match app.stats_view {
            StatsView::Import => {
                "StatsSym — Import (like BioSym Import)\n\
                 Close help:  Esc  or  Alt-?\n\n\
                 \n\
                 FILES\n\n\
                   ↑ ↓                 navigate discovered files\n\
                   Enter               load selected (or typed path) → Lab\n\
                   /                   filter mode\n\
                   x                   delete selected (y confirm / n Esc cancel)\n\
                   type…               manual path (Esc clears)\n\
                   Ctrl+R  /  F5       refresh discovery\n\n\
                 Numeric CSV with headers; non-numeric columns skipped.\n\n\
                 \n\
                 GENERATE\n\n\
                   Ctrl+G              open Generate tab (presets)\n\
                   Ctrl+← / Ctrl+→     Import · Lab · Generate\n\
                   Ctrl+1/2/3          jump Import / Lab / Generate\n\n\
                 \n\
                 GLOBAL\n\n\
                   Ctrl+H              Home\n\
                   Esc Esc / Ctrl+Q    quit (at Import root)\n"
            }
            StatsView::Lab => {
                "StatsSym — Lab (workspace, like BioSym Explore)\n\
                 Close help:  Esc  or  Alt-?\n\n\
                 \n\
                 After Import or Generate you land here with the table loaded.\n\
                 Column names show in the header and status (x/y cycles names).\n\n\
                 TASKS\n\n\
                   t / T               cycle task\n\
                   1–6                 Describe · Correlate · Fit OLS · Fit Poly\n\
                                       · Classify · Pipeline\n\
                   x / X               feature (X) column − / +\n\
                   y / Y               target (Y) column − / +\n\
                   Enter               run analysis\n\
                   h                   residual panel: Bland–Altman ↔ histogram\n\
                   Esc                 back to Import\n\
                   Ctrl+←→             module tabs\n\n\
                 FIT POLY\n\n\
                   Degree search 0..=max (crate polyreg)\n\
                   Left: degree table  R²  adjR²  AIC\n\
                   ★ = min AIC (preferred)  ·  ☆ = max R² if different\n\
                   Focus row note: nested χ² vs d−1 + p  ·  RMSE  ·  BIC  ·  β\n\
                   Right: fit + residuals for ▶ focused degree\n\
                   Under table: best-by-AIC summary\n\
                   ↑ ↓ / f             focus degree (plots follow)\n\
                   d / D               max degree +/−  (Enter re-run)\n\n\
                 CLASSIFY\n\n\
                   logistic binary (2 classes) or OVR multiclass (3+)\n\
                   y rounded to integer labels · X = all other cols\n\
                   plot: P(class) or confidence · confusion in summary\n\
                   demos: TwoClassBlobs · ThreeClassBlobs\n\n\
                 PIPELINE\n\n\
                   Left: splits table  ·  Right: plots for ★ row\n\
                   m / M               model OLS ↔ Logistic\n\
                   k / K               folds −/+  (Enter re-run)\n\
                   ↑ ↓ / f             focus split\n\
                   OLS: R² / RMSE / MAE · ŷ vs y\n\
                   Logistic: Acc / bal_acc / macro-F1 · true vs pred\n\
                   3-group story: ThreeClassBlobs → Classify → Pipeline+m Logistic\n"
            }
            StatsView::Generate => {
                "StatsSym — Generate synthetic data\n\
                 Close help:  Esc  or  Alt-?\n\n\
                 \n\
                 Open via Ctrl+G or Ctrl+→ from Lab / Ctrl+3.\n\n\
                 PRESETS  (symworx-stats::synthetic)\n\n\
                   ↑ ↓                 select preset\n\
                   n / N               sample size − / +\n\
                   s / S               seed − / +\n\
                   + / −               noise − / +\n\
                   Enter               generate CSV → load → jump to Lab\n\
                   Esc                 back to Import\n\n\
                 Linear regression → Lab task Regress; bivariate → Correlate;\n\
                 others → Describe. Press Enter in Lab to run.\n"
            }
        };
        frame.render_widget(
            Paragraph::new(body).block(
                Block::new()
                    .borders(Borders::ALL)
                    .padding(Padding::horizontal(1))
                    .title(" Help — StatsSym "),
            ),
            area,
        );
        return;
    }

    match app.stats_view {
        StatsView::Import => import::render_import(frame, app, area),
        StatsView::Lab => {
            let outer = Block::new()
                .title(" StatsSym — Lab ")
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .border_style(Color::Magenta);
            let inner = outer.inner(area);
            frame.render_widget(outer, area);
            lab::render_lab(frame, app, inner);
        }
        StatsView::Generate => {
            let outer = Block::new()
                .title(" StatsSym — Generate ")
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .border_style(Color::Magenta);
            let inner = outer.inner(area);
            frame.render_widget(outer, area);
            generate::render_generate(frame, app, inner);
        }
    }
}
