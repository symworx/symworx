// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Top-level tabs and workflow paths.

/// Top-level tabs for the application.
/// BioSym order (footer): Import · Explore · Dynamics · Generate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Import,
    Explore,
    Dynamics,
    /// BioSym synthetic demo generator (Ctrl+G; also a module tab).
    Generate,
    Spatial,
    LoadSym,
    /// StatsSym workflow surface (internal sub-views; not BioSym tab bar).
    Stats,
    // Home is special: when active (via workflow) we render a full landing instead of the bar+content
    Home,
}

impl Tab {
    /// BioSym sub-tabs in footer / Ctrl+←→ order.
    pub const BIOSYM_TABS: [Tab; 4] = [Tab::Import, Tab::Explore, Tab::Dynamics, Tab::Generate];

    pub fn title(self) -> &'static str {
        match self {
            Tab::Import => "Import",
            Tab::Explore => "Explore",
            Tab::Dynamics => "Dynamics",
            Tab::Generate => "Generate",
            Tab::Spatial => "Spatial",
            Tab::LoadSym => "LoadSym",
            Tab::Stats => "StatsSym",
            Tab::Home => "Home",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Tab::Import => 0,
            Tab::Explore => 1,
            Tab::Dynamics => 2,
            Tab::Generate => 3,
            Tab::Spatial => 4,
            Tab::LoadSym => 5,
            Tab::Stats => 6,
            Tab::Home => 0,
        }
    }

    /// Previous BioSym tab (clamped).
    pub fn biosym_prev(self) -> Tab {
        let i = Self::BIOSYM_TABS.iter().position(|&t| t == self).unwrap_or(0);
        if i == 0 { Tab::Import } else { Self::BIOSYM_TABS[i - 1] }
    }

    /// Next BioSym tab (clamped).
    pub fn biosym_next(self) -> Tab {
        let i = Self::BIOSYM_TABS.iter().position(|&t| t == self).unwrap_or(0);
        Self::BIOSYM_TABS[(i + 1).min(Self::BIOSYM_TABS.len() - 1)]
    }
}

pub fn tab_titles() -> Vec<ratatui::text::Span<'static>> {
    vec!["1: Import", "2: Explore", "3: Dynamics", "G: Generate"]
        .into_iter()
        .map(ratatui::text::Span::from)
        .collect()
}

/// High-level analysis workflow / path. Drives landing + context for sub-modes and tab adaptation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Workflow {
    #[default]
    Home,
    BioSym,
    SpatialSym,
    LoadSym,
    /// Tabular statistics lab (students + research).
    StatsSym,
}
