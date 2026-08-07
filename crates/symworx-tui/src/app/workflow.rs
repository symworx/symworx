// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Workflow switching and status defaults.

use super::{
    App,
    LoadSymView,
    SpatialView,
    StatsView,
    Tab,
    Workflow,
};

impl App {
    pub fn switch_workflow(&mut self, wf: Workflow) {
        self.current_workflow = wf;
        match wf {
            Workflow::Home => {
                self.current_tab = Tab::Home;
                self.status = "Home — 1 BioSym  2 StatsSym  3 LoadSym  4 Spatial  • Ctrl+H here".to_string();
            }
            Workflow::BioSym => {
                self.current_tab = if self.loaded_signal.is_some() {
                    Tab::Explore
                } else {
                    Tab::Import
                };
                self.status = if self.loaded_signal.is_some() {
                    "BioSym Explore — Ctrl+←→ tabs  ·  Ctrl+G generate  ·  Ctrl+L live".into()
                } else {
                    "BioSym Import — ↑↓ files  Enter load  Ctrl+G generate  ·  Ctrl+←→ tabs".into()
                };
            }
            Workflow::SpatialSym => {
                self.current_tab = Tab::Spatial;
                self.spatial_view = SpatialView::Visualize;
                self.status =
                    "SpatialSym — g:regen  i:import/generate  arrows:nav  (sub-views inside Spatial tab)".to_string();
            }
            Workflow::LoadSym => {
                self.current_tab = Tab::LoadSym;
                self.loadsym_view = LoadSymView::List;
                self.loadsym_selection = 0;
                // Refresh catalog if empty (or always try when entering workflow)
                let _ = crate::processing::try_load_loadsym_catalog(self);
                let cat = if self.loadsym_from_catalog {
                    format!("catalog {} days", self.daily_loads.len())
                } else {
                    "no catalog (g=demo, or symload ingest)".to_string()
                };
                self.status = format!(
                    "LoadSym — 1 Workout  2 Metrics  3 Calendar  4 Optimization  • {}  • Ctrl+H home",
                    cat
                );
            }
            Workflow::StatsSym => {
                self.current_tab = Tab::Stats;
                // Like BioSym: go to workspace if data already loaded, else Import.
                self.stats_view = if self.stats_table.is_some() {
                    StatsView::Lab
                } else {
                    StatsView::Import
                };
                self.stats_selection = 0;
                self.status = match self.stats_view {
                    StatsView::Lab => {
                        if let Some(ref tab) = self.stats_table {
                            format!(
                                "StatsSym Lab — {}×{}  ·  1–4 task  x/y cols  Enter run  ·  Ctrl+←→ tabs",
                                tab.n_rows(),
                                tab.n_cols()
                            )
                        } else {
                            "StatsSym Lab — load a table first (Import or Ctrl+G)".into()
                        }
                    }
                    _ => "StatsSym Import — ↑↓ files  Enter load  / filter  Ctrl+G generate  ·  Ctrl+←→ tabs".into(),
                };
            }
        }
        self.ensure_status_for_current_tab();
    }
    pub fn ensure_status_for_current_tab(&mut self) {
        if self.current_workflow == Workflow::Home {
            if !self.status.starts_with("Home") {
                self.status = "Home — 1=BioSym  2=StatsSym  3=LoadSym  4=Spatial".to_string();
            }
            return;
        }
        if self.current_tab != Tab::Spatial && self.status.starts_with("Spatial") {
            self.status = match self.current_tab {
                Tab::Import => "Import — / filter, ↑↓ select, Enter load, c convert, Ctrl+G generate".to_string(),
                Tab::Explore => {
                    "Explore — Ctrl+L live  p process  k peaks  K params  i tachogram  e export".to_string()
                }
                Tab::Dynamics => "Dynamics (RQA/cRQA + MSE)".to_string(),
                Tab::Generate => "Generate — ↑↓ preset  Enter → Explore  ·  1/2/3 quick".to_string(),
                _ => "Symview".to_string(),
            };
        } else if self.current_tab == Tab::Spatial && !self.status.starts_with("Spatial") {
            let maxf = self
                .spatial_batch
                .as_ref()
                .map(|b| b.num_times().saturating_sub(1))
                .unwrap_or(0);
            self.status = format!("Spatial: frame {}/{}", self.spatial_frame_idx, maxf);
        } else if self.current_tab == Tab::LoadSym && self.loadsym_view == LoadSymView::List {
            self.status = "LoadSym — ↑↓ 1–4 Workout · Metrics · Calendar · Optimization  • Esc back".to_string();
        }
    }
}
